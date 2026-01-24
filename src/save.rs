use anyhow::{Context, Result};
use notify_rust::Notification;
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[cfg(feature = "grim")]
#[allow(clippy::too_many_arguments)]
pub fn save_geometry_with_grim(
    geometry: &str,
    save_fullpath: &PathBuf,
    clipboard_only: bool,
    raw: bool,
    command: Option<Vec<String>>,
    silent: bool,
    notif_timeout: u32,
    debug: bool,
) -> Result<()> {
    use std::io::Write;

    if debug {
        eprintln!("Saving geometry with grim-rs library: {}", geometry);
    }

    let region: grim_rs::Box = geometry.parse().context(format!(
        "Failed to parse geometry '{}' into grim-rs::Box",
        geometry
    ))?;

    let mut grim = grim_rs::Grim::new().context("Failed to initialize grim-rs")?;

    let capture_result = grim
        .capture_region(region)
        .context("Failed to capture screenshot region")?;

    if raw {
        let png_bytes = grim
            .to_png(
                capture_result.data(),
                capture_result.width(),
                capture_result.height(),
            )
            .context("Failed to encode screenshot as PNG")?;
        std::io::stdout().write_all(&png_bytes)?;
        return Ok(());
    }

    if !clipboard_only {
        create_dir_all(save_fullpath.parent().unwrap())
            .context("Failed to create screenshot directory")?;

        grim.save_png(
            capture_result.data(),
            capture_result.width(),
            capture_result.height(),
            save_fullpath,
        )
        .context(format!(
            "Failed to save screenshot to '{}'",
            save_fullpath.display()
        ))?;

        let wl_copy_status = Command::new("wl-copy")
            .arg("--type")
            .arg("image/png")
            .stdin(std::fs::File::open(save_fullpath).context(format!(
                "Failed to open screenshot file '{}'",
                save_fullpath.display()
            ))?)
            .status()
            .context("Failed to run wl-copy")?;
        if !wl_copy_status.success() {
            return Err(anyhow::anyhow!("wl-copy failed to copy screenshot"));
        }

        if let Some(cmd) = command {
            let cmd_status = Command::new(&cmd[0])
                .args(&cmd[1..])
                .arg(save_fullpath)
                .status()
                .context(format!("Failed to run command '{}'", cmd[0]))?;
            if !cmd_status.success() {
                return Err(anyhow::anyhow!("Command '{}' failed", cmd[0]));
            }
        }
    } else {
        let png_bytes = grim
            .to_png(
                capture_result.data(),
                capture_result.width(),
                capture_result.height(),
            )
            .context("Failed to encode screenshot as PNG")?;

        let mut wl_copy = Command::new("wl-copy")
            .arg("--type")
            .arg("image/png")
            .stdin(Stdio::piped())
            .spawn()
            .context("Failed to start wl-copy")?;
        wl_copy
            .stdin
            .as_mut()
            .unwrap()
            .write_all(&png_bytes)
            .context("Failed to write to wl-copy stdin")?;
        let wl_copy_status = wl_copy.wait().context("Failed to wait for wl-copy")?;
        if !wl_copy_status.success() {
            return Err(anyhow::anyhow!("wl-copy failed to copy screenshot"));
        }
    }

    if !silent {
        let message = if clipboard_only {
            "Image copied to the clipboard".to_string()
        } else {
            format!(
                "Image saved in <i>{}</i> and copied to the clipboard.",
                save_fullpath.display()
            )
        };
        Notification::new()
            .summary("Screenshot saved")
            .body(&message)
            .icon(save_fullpath.to_str().unwrap_or("screenshot"))
            .timeout(notif_timeout as i32)
            .appname("Hyprshot-rs")
            .show()
            .context("Failed to show notification")?;
    }

    Ok(())
}

#[cfg(feature = "native")]
pub fn save_geometry_with_native(
    geometry: &str,
    save_fullpath: &PathBuf,
    clipboard_only: bool,
    raw: bool,
    command: Option<Vec<String>>,
    silent: bool,
    notif_timeout: u32,
    debug: bool,
) -> Result<()> {
    use image::{DynamicImage, ImageBuffer, Rgba};
    use wayland_client::{
        Connection, Dispatch, QueueHandle,
        protocol::{wl_compositor::WlCompositor, wl_output::WlOutput, wl_shm::WlShm},
    };
    use wayland_protocols::unstable::screencopy::v1::client::{
        zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
        zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
    };

    if debug {
        eprintln!("Saving geometry with native Wayland: {}", geometry);
    }

    let parts: Vec<&str> = geometry.split(' ').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!("Invalid geometry format: '{}'", geometry));
    }
    let xy: Vec<&str> = parts[0].split(',').collect();
    let wh: Vec<&str> = parts[1].split('x').collect();
    let x: i32 = xy[0].parse().context("Invalid x coordinate")?;
    let y: i32 = xy[1].parse().context("Invalid y coordinate")?;
    let width: i32 = wh[0].parse().context("Invalid width")?;
    let height: i32 = wh[1].parse().context("Invalid height")?;

    let conn = Connection::connect_to_env().context("Failed to connect to Wayland")?;
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let display = conn.display();
    let globals = conn
        .get_registry(&qh, ())
        .context("Failed to get Wayland registry")?;

    struct State {
        compositor: Option<WlCompositor>,
        shm: Option<WlShm>,
        screencopy_manager: Option<ZwlrScreencopyManagerV1>,
        outputs: Vec<WlOutput>,
    }

    impl Dispatch<wayland_client::protocol::wl_registry::WlRegistry, ()> for State {
        fn event(
            &mut self,
            registry: &wayland_client::protocol::wl_registry::WlRegistry,
            event: wayland_client::protocol::wl_registry::Event,
            _: &(),
            _: &Connection,
            qh: &QueueHandle<Self>,
        ) {
            if let wayland_client::protocol::wl_registry::Event::Global {
                name,
                interface,
                version,
            } = event
            {
                match interface.as_str() {
                    "wl_compositor" => {
                        self.compositor =
                            Some(registry.bind::<WlCompositor, _, _>(name, version, qh, ()));
                    }
                    "wl_shm" => {
                        self.shm = Some(registry.bind::<WlShm, _, _>(name, version, qh, ()));
                    }
                    "zwlr_screencopy_manager_v1" => {
                        self.screencopy_manager = Some(
                            registry.bind::<ZwlrScreencopyManagerV1, _, _>(name, version, qh, ()),
                        );
                    }
                    "wl_output" => {
                        self.outputs
                            .push(registry.bind::<WlOutput, _, _>(name, version, qh, ()));
                    }
                    _ => {}
                }
            }
        }
    }

    let mut state = State {
        compositor: None,
        shm: None,
        screencopy_manager: None,
        outputs: vec![],
    };

    event_queue
        .roundtrip(&mut state)
        .context("Failed to initialize Wayland globals")?;

    let screencopy_manager = state
        .screencopy_manager
        .context("wlr-screencopy-unstable-v1 not available")?;
    let output = state.outputs.get(0).context("No outputs found")?;

    let frame = screencopy_manager.capture_output_region(0, output, x, y, width, height, &qh, ());

    struct FrameState {
        buffer: Option<Vec<u8>>,
        width: u32,
        height: u32,
        format: Option<wayland_client::protocol::wl_shm::Format>,
    }

    impl Dispatch<ZwlrScreencopyFrameV1, ()> for FrameState {
        fn event(
            &mut self,
            frame: &ZwlrScreencopyFrameV1,
            event: wayland_protocols::unstable::screencopy::v1::client::zwlr_screencopy_frame_v1::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            match event {
                zwlr_screencopy_frame_v1::Event::Buffer {
                    format,
                    width,
                    height,
                    stride,
                } => {
                    self.width = width;
                    self.height = height;
                    self.format = Some(format);
                    self.buffer = Some(vec![0u8; (stride * height) as usize]);
                }
                zwlr_screencopy_frame_v1::Event::Ready { .. } => {
                    frame.destroy();
                }
                _ => {}
            }
        }
    }

    let mut frame_state = FrameState {
        buffer: None,
        width: 0,
        height: 0,
        format: None,
    };

    event_queue
        .roundtrip(&mut frame_state)
        .context("Failed to capture frame")?;

    let buffer = frame_state
        .buffer
        .context("Failed to receive frame buffer")?;
    let width = frame_state.width;
    let height = frame_state.height;

    let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(width, height, buffer)
        .context("Failed to create image from buffer")?;
    let dynamic_img = DynamicImage::ImageRgba8(img);

    if raw {
        let mut stdout = std::io::stdout();
        dynamic_img
            .write_to(&mut stdout, image::ImageOutputFormat::Png)
            .context("Failed to write raw image to stdout")?;
        return Ok(());
    }

    if !clipboard_only {
        create_dir_all(save_fullpath.parent().unwrap())
            .context("Failed to create screenshot directory")?;
        dynamic_img.save(save_fullpath).context(format!(
            "Failed to save screenshot to '{}'",
            save_fullpath.display()
        ))?;

        let wl_copy_status = Command::new("wl-copy")
            .arg("--type")
            .arg("image/png")
            .stdin(std::fs::File::open(save_fullpath).context(format!(
                "Failed to open screenshot file '{}'",
                save_fullpath.display()
            ))?)
            .status()
            .context("Failed to run wl-copy")?;
        if !wl_copy_status.success() {
            return Err(anyhow::anyhow!("wl-copy failed to copy screenshot"));
        }

        if let Some(cmd) = command {
            let cmd_status = Command::new(&cmd[0])
                .args(&cmd[1..])
                .arg(save_fullpath)
                .status()
                .context(format!("Failed to run command '{}'", cmd[0]))?;
            if !cmd_status.success() {
                return Err(anyhow::anyhow!("Command '{}' failed", cmd[0]));
            }
        }
    } else {
        let mut buffer = Vec::new();
        dynamic_img
            .write_to(
                &mut std::io::Cursor::new(&mut buffer),
                image::ImageOutputFormat::Png,
            )
            .context("Failed to encode image to PNG")?;

        let mut wl_copy = Command::new("wl-copy")
            .arg("--type")
            .arg("image/png")
            .stdin(Stdio::piped())
            .spawn()
            .context("Failed to start wl-copy")?;
        wl_copy
            .stdin
            .as_mut()
            .unwrap()
            .write_all(&buffer)
            .context("Failed to write to wl-copy stdin")?;
        let wl_copy_status = wl_copy.wait().context("Failed to wait for wl-copy")?;
        if !wl_copy_status.success() {
            return Err(anyhow::anyhow!("wl-copy failed to copy screenshot"));
        }
    }

    if !silent {
        let message = if clipboard_only {
            "Image copied to the clipboard".to_string()
        } else {
            format!(
                "Image saved in <i>{}</i> and copied to the clipboard.",
                save_fullpath.display()
            )
        };
        Notification::new()
            .summary("Screenshot saved")
            .body(&message)
            .icon(save_fullpath.to_str().unwrap_or("screenshot"))
            .timeout(notif_timeout as i32)
            .appname("Hyprshot-rs")
            .show()
            .context("Failed to show notification")?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn save_geometry(
    geometry: &str,
    save_fullpath: &PathBuf,
    clipboard_only: bool,
    raw: bool,
    command: Option<Vec<String>>,
    silent: bool,
    notif_timeout: u32,
    debug: bool,
) -> Result<()> {
    #[cfg(feature = "grim")]
    return save_geometry_with_grim(
        geometry,
        save_fullpath,
        clipboard_only,
        raw,
        command,
        silent,
        notif_timeout,
        debug,
    );
    #[cfg(feature = "native")]
    return save_geometry_with_native(
        geometry,
        save_fullpath,
        clipboard_only,
        raw,
        command,
        silent,
        notif_timeout,
        debug,
    );
    #[cfg(not(any(feature = "grim", feature = "native")))]
    compile_error!("At least one of 'grim' or 'native' features must be enabled");
}
