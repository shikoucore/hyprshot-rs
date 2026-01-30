use anyhow::{Context, Result};

#[cfg(all(target_os = "linux", feature = "freeze"))]
mod imp {
    use super::*;
    use grim_rs::Grim;
    use std::{
        collections::HashMap,
        os::fd::{AsRawFd, BorrowedFd},
        sync::mpsc,
        thread,
        time::Duration,
    };
    use wayland_client::{
        Connection, Dispatch, QueueHandle,
        protocol::{
            wl_buffer::WlBuffer,
            wl_compositor::WlCompositor,
            wl_output::Mode as WlOutputMode,
            wl_output::WlOutput,
            wl_registry::WlRegistry,
            wl_shm::{self, WlShm},
            wl_shm_pool::WlShmPool,
            wl_surface::WlSurface,
        },
    };
    use wayland_protocols::xdg::xdg_output::zv1::client::{
        zxdg_output_manager_v1::ZxdgOutputManagerV1, zxdg_output_v1::ZxdgOutputV1,
    };
    use wayland_protocols_wlr::layer_shell::v1::client::{
        zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
        zwlr_layer_surface_v1::{Anchor, KeyboardInteractivity, ZwlrLayerSurfaceV1},
    };

    pub struct FreezeGuard {
        stop_tx: mpsc::Sender<()>,
        join: Option<thread::JoinHandle<Result<()>>>,
    }

    impl FreezeGuard {
        pub fn stop(mut self) -> Result<()> {
            let _ = self.stop_tx.send(());
            if let Some(join) = self.join.take() {
                return join
                    .join()
                    .unwrap_or_else(|_| Err(anyhow::anyhow!("Freeze thread panicked")));
            }
            Ok(())
        }
    }

    impl Drop for FreezeGuard {
        fn drop(&mut self) {
            let _ = self.stop_tx.send(());
            if let Some(join) = self.join.take() {
                let _ = join.join();
            }
        }
    }

    #[derive(Clone)]
    struct CaptureImage {
        name: String,
        geom: (i32, i32, i32, i32),
        data: Vec<u8>,
        width: u32,
        height: u32,
        scale: i32,
    }

    pub fn start_freeze(selected_output: Option<&str>, debug: bool) -> Result<FreezeGuard> {
        let captures = match capture_outputs(selected_output, debug) {
            Ok(captures) => captures,
            Err(err) if is_missing_screencopy(&err) => {
                // FIXME: нужно проверить поддержку wlr-screencopy на Hyprland/Sway/River/Wayfire.
                eprintln!(
                    "Freeze is disabled: compositor does not support wlr-screencopy. \
Check the support for this protocol on Hyprland/Sway/River/Wayfire."
                );
                let (stop_tx, _stop_rx) = mpsc::channel();
                return Ok(FreezeGuard {
                    stop_tx,
                    join: None,
                });
            }
            Err(err) => return Err(err),
        };

        let (stop_tx, stop_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();

        let join = thread::spawn(move || run_freeze(captures, stop_rx, ready_tx, debug));

        match ready_rx.recv_timeout(Duration::from_secs(2)) {
            Ok(Ok(())) => Ok(FreezeGuard {
                stop_tx,
                join: Some(join),
            }),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(anyhow::anyhow!("Freeze overlay initialization timed out")),
        }
    }

    fn capture_outputs(selected_output: Option<&str>, debug: bool) -> Result<Vec<CaptureImage>> {
        let mut grim = Grim::new().context("Failed to initialize grim-rs")?;
        let outputs = grim.get_outputs().context("Failed to list outputs")?;

        let targets: Vec<_> = if let Some(name) = selected_output {
            let matched: Vec<_> = outputs.iter().filter(|o| o.name() == name).collect();
            if matched.is_empty() {
                return Err(anyhow::anyhow!("Output '{}' not found", name));
            }
            matched
        } else {
            outputs.iter().collect()
        };

        let mut captures = Vec::new();
        for output in targets {
            let name = output.name().to_string();
            let scale = output.scale().max(1);
            let geometry = output.geometry();
            let capture = grim
                .capture_output(&name)
                .with_context(|| format!("Failed to capture output '{}'", name))?;

            if debug {
                eprintln!(
                    "Freeze capture: {} ({}x{})",
                    name,
                    capture.width(),
                    capture.height()
                );
            }

            let width = capture.width();
            let height = capture.height();
            captures.push(CaptureImage {
                name,
                geom: (
                    geometry.x(),
                    geometry.y(),
                    geometry.width(),
                    geometry.height(),
                ),
                data: capture.into_data(),
                width,
                height,
                scale,
            });
        }

        if captures.is_empty() {
            return Err(anyhow::anyhow!("No outputs available for freeze"));
        }

        Ok(captures)
    }

    #[derive(Debug)]
    struct OutputKey(usize);

    #[derive(Debug)]
    struct SurfaceKey(usize);

    struct OutputEntry {
        output: WlOutput,
        name: Option<String>,
        xdg_output: Option<ZxdgOutputV1>,
        pos_x: Option<i32>,
        pos_y: Option<i32>,
        mode_width: Option<i32>,
        mode_height: Option<i32>,
        scale: i32,
        logical_x: Option<i32>,
        logical_y: Option<i32>,
        logical_width: Option<i32>,
        logical_height: Option<i32>,
    }

    struct SurfaceEntry {
        surface: WlSurface,
        layer_surface: ZwlrLayerSurfaceV1,
        buffer: WlBuffer,
        _tmp: tempfile::NamedTempFile,
        _mmap: memmap2::MmapMut,
        configured: bool,
    }

    struct State {
        compositor: Option<WlCompositor>,
        shm: Option<WlShm>,
        layer_shell: Option<ZwlrLayerShellV1>,
        xdg_output_manager: Option<ZxdgOutputManagerV1>,
        outputs: Vec<OutputEntry>,
        surfaces: Vec<SurfaceEntry>,
    }

    impl Dispatch<WlRegistry, ()> for State {
        fn event(
            state: &mut Self,
            registry: &WlRegistry,
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
                        state.compositor = Some(registry.bind(name, version.min(5), qh, ()));
                    }
                    "wl_shm" => {
                        state.shm = Some(registry.bind(name, version.min(1), qh, ()));
                    }
                    "zwlr_layer_shell_v1" => {
                        state.layer_shell = Some(registry.bind(name, version.min(4), qh, ()));
                    }
                    "wl_output" => {
                        let idx = state.outputs.len();
                        let output = registry.bind::<WlOutput, _, _>(
                            name,
                            version.min(4),
                            qh,
                            OutputKey(idx),
                        );
                        state.outputs.push(OutputEntry {
                            output,
                            name: None,
                            xdg_output: None,
                            pos_x: None,
                            pos_y: None,
                            mode_width: None,
                            mode_height: None,
                            scale: 1,
                            logical_x: None,
                            logical_y: None,
                            logical_width: None,
                            logical_height: None,
                        });
                    }
                    "zxdg_output_manager_v1" => {
                        state.xdg_output_manager =
                            Some(registry.bind(name, version.min(3), qh, ()));
                    }
                    _ => {}
                }
            }
        }
    }

    impl Dispatch<WlOutput, OutputKey> for State {
        fn event(
            state: &mut Self,
            _: &WlOutput,
            event: wayland_client::protocol::wl_output::Event,
            data: &OutputKey,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            let Some(entry) = state.outputs.get_mut(data.0) else {
                return;
            };
            match event {
                wayland_client::protocol::wl_output::Event::Geometry { x, y, .. } => {
                    entry.pos_x = Some(x);
                    entry.pos_y = Some(y);
                }
                wayland_client::protocol::wl_output::Event::Mode {
                    flags,
                    width,
                    height,
                    ..
                } => {
                    let is_current = match flags {
                        wayland_client::WEnum::Value(f) => f.contains(WlOutputMode::Current),
                        wayland_client::WEnum::Unknown(_) => false,
                    };
                    if is_current {
                        entry.mode_width = Some(width);
                        entry.mode_height = Some(height);
                    }
                }
                wayland_client::protocol::wl_output::Event::Scale { factor } => {
                    entry.scale = factor.max(1);
                }
                wayland_client::protocol::wl_output::Event::Name { name } => {
                    entry.name = Some(name);
                }
                _ => {}
            }
        }
    }

    impl Dispatch<ZxdgOutputV1, OutputKey> for State {
        fn event(
            state: &mut Self,
            _: &ZxdgOutputV1,
            event: wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_v1::Event,
            data: &OutputKey,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            let Some(entry) = state.outputs.get_mut(data.0) else {
                return;
            };
            match event {
                wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_v1::Event::LogicalPosition { x, y } => {
                    entry.logical_x = Some(x);
                    entry.logical_y = Some(y);
                }
                wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_v1::Event::LogicalSize { width, height } => {
                    entry.logical_width = Some(width);
                    entry.logical_height = Some(height);
                }
                wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_v1::Event::Name {
                    name,
                } => {
                    entry.name = Some(name);
                }
                _ => {}
            }
        }
    }

    impl Dispatch<ZwlrLayerSurfaceV1, SurfaceKey> for State {
        fn event(
            state: &mut Self,
            surface: &ZwlrLayerSurfaceV1,
            event: wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Event,
            data: &SurfaceKey,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            if let Some(entry) = state.surfaces.get_mut(data.0) {
                match event {
                    wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Event::Configure {
                        serial,
                        ..
                    } => {
                        surface.ack_configure(serial);
                        entry.configured = true;
                    }
                    wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Event::Closed => {
                        entry.configured = false;
                    }
                    _ => {}
                }
            }
        }
    }

    impl Dispatch<WlCompositor, ()> for State {
        fn event(
            _: &mut Self,
            _: &WlCompositor,
            _: wayland_client::protocol::wl_compositor::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<WlShm, ()> for State {
        fn event(
            _: &mut Self,
            _: &WlShm,
            _: wayland_client::protocol::wl_shm::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<WlShmPool, ()> for State {
        fn event(
            _: &mut Self,
            _: &WlShmPool,
            _: wayland_client::protocol::wl_shm_pool::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<WlSurface, ()> for State {
        fn event(
            _: &mut Self,
            _: &WlSurface,
            _: wayland_client::protocol::wl_surface::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<WlBuffer, ()> for State {
        fn event(
            _: &mut Self,
            _: &WlBuffer,
            _: wayland_client::protocol::wl_buffer::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<ZwlrLayerShellV1, ()> for State {
        fn event(
            _: &mut Self,
            _: &ZwlrLayerShellV1,
            _: wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<ZxdgOutputManagerV1, ()> for State {
        fn event(
            _: &mut Self,
            _: &ZxdgOutputManagerV1,
            _: wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_manager_v1::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    fn run_freeze(
        captures: Vec<CaptureImage>,
        stop_rx: mpsc::Receiver<()>,
        ready_tx: mpsc::Sender<Result<()>>,
        debug: bool,
    ) -> Result<()> {
        let conn = Connection::connect_to_env().context("Failed to connect to Wayland")?;
        let mut event_queue = conn.new_event_queue();
        let qh = event_queue.handle();

        let registry = conn.display().get_registry(&qh, ());

        let mut state = State {
            compositor: None,
            shm: None,
            layer_shell: None,
            xdg_output_manager: None,
            outputs: Vec::new(),
            surfaces: Vec::new(),
        };

        event_queue
            .roundtrip(&mut state)
            .context("Failed to initialize Wayland globals")?;

        if let Some(manager) = &state.xdg_output_manager {
            for (idx, entry) in state.outputs.iter_mut().enumerate() {
                let xdg_output = manager.get_xdg_output(&entry.output, &qh, OutputKey(idx));
                entry.xdg_output = Some(xdg_output);
            }
            event_queue
                .roundtrip(&mut state)
                .context("Failed to receive output names")?;
        }

        let compositor = state
            .compositor
            .as_ref()
            .context("wl_compositor not available")?;
        let shm = state.shm.as_ref().context("wl_shm not available")?;
        let Some(layer_shell) = state.layer_shell.as_ref() else {
            // FIXME: нужно проверить поддержку wlr-layer-shell на Hyprland/Sway/River/Wayfire.
            eprintln!(
                "Freeze is disabled: compositor does not support wlr-layer-shell. \
Check the support for this protocol on Hyprland/Sway/River/Wayfire."
            );
            let _ = ready_tx.send(Ok(()));
            return Ok(());
        };

        let capture_by_name: HashMap<&str, &CaptureImage> =
            captures.iter().map(|c| (c.name.as_str(), c)).collect();

        for output in &state.outputs {
            let capture = if let Some(name) = output.name.as_deref() {
                capture_by_name.get(name).copied()
            } else {
                None
            }
            .or_else(|| {
                output_geometry(output).and_then(|geom| {
                    captures
                        .iter()
                        .find(|capture| geometry_close(capture.geom, geom))
                })
            });

            let Some(capture) = capture else {
                if debug {
                    eprintln!(
                        "Freeze: output '{}' не сопоставлен с захватом",
                        output.name.as_deref().unwrap_or("<unknown>")
                    );
                }
                continue;
            };

            let surface_idx = state.surfaces.len();
            let surface = compositor.create_surface(&qh, ());
            let layer_surface = layer_shell.get_layer_surface(
                &surface,
                Some(&output.output),
                Layer::Overlay,
                "hyprshot-freeze".to_string(),
                &qh,
                SurfaceKey(surface_idx),
            );

            layer_surface.set_anchor(Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right);
            layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
            layer_surface.set_exclusive_zone(-1);

            if capture.scale > 1 {
                surface.set_buffer_scale(capture.scale);
            }

            surface.commit();

            let (buffer, tmp, mmap) = create_buffer(shm, &qh, capture).with_context(|| {
                format!(
                    "Failed to create buffer for output '{}'",
                    output.name.as_deref().unwrap_or("<unknown>")
                )
            })?;

            state.surfaces.push(SurfaceEntry {
                surface,
                layer_surface,
                buffer,
                _tmp: tmp,
                _mmap: mmap,
                configured: false,
            });
        }

        if state.surfaces.is_empty() {
            let _ = ready_tx.send(Err(anyhow::anyhow!(
                "No matching outputs found for freeze overlay"
            )));
            return Ok(());
        }

        event_queue
            .roundtrip(&mut state)
            .context("Failed to configure freeze surfaces")?;

        for entry in &state.surfaces {
            entry.surface.attach(Some(&entry.buffer), 0, 0);
            entry.surface.commit();
        }
        conn.flush().ok();

        let _ = ready_tx.send(Ok(()));

        loop {
            if stop_rx.try_recv().is_ok() {
                break;
            }
            event_queue.roundtrip(&mut state).ok();
        }

        if debug {
            eprintln!("Freeze overlay stopped");
        }

        for entry in state.surfaces {
            entry.layer_surface.destroy();
            entry.surface.destroy();
            entry.buffer.destroy();
        }
        drop(registry);

        Ok(())
    }

    fn create_buffer(
        shm: &WlShm,
        qh: &QueueHandle<State>,
        capture: &CaptureImage,
    ) -> Result<(WlBuffer, tempfile::NamedTempFile, memmap2::MmapMut)> {
        let width = capture.width as i32;
        let height = capture.height as i32;
        let stride = width * 4;
        let size = (stride * height) as usize;

        let mut tmp_file = tempfile::NamedTempFile::new()
            .context("Failed to create temporary file for shm buffer")?;
        tmp_file
            .as_file_mut()
            .set_len(size as u64)
            .context("Failed to resize shm buffer file")?;

        let mut mmap = unsafe {
            memmap2::MmapMut::map_mut(&tmp_file).context("Failed to memory-map shm buffer")?
        };

        let src = &capture.data;
        let dst = &mut mmap[..];
        for (i, px) in src.chunks_exact(4).enumerate() {
            let offset = i * 4;
            dst[offset] = px[2];
            dst[offset + 1] = px[1];
            dst[offset + 2] = px[0];
            dst[offset + 3] = px[3];
        }

        let pool = shm.create_pool(
            unsafe { BorrowedFd::borrow_raw(tmp_file.as_file().as_raw_fd()) },
            size as i32,
            qh,
            (),
        );
        let buffer = pool.create_buffer(0, width, height, stride, wl_shm::Format::Argb8888, qh, ());
        pool.destroy();

        Ok((buffer, tmp_file, mmap))
    }

    fn output_geometry(output: &OutputEntry) -> Option<(i32, i32, i32, i32)> {
        let x = output.logical_x.or(output.pos_x)?;
        let y = output.logical_y.or(output.pos_y)?;
        let width = if let Some(width) = output.logical_width {
            width
        } else {
            let mode_width = output.mode_width?;
            let scale = output.scale.max(1);
            ((mode_width as f64) / (scale as f64)).round() as i32
        };
        let height = if let Some(height) = output.logical_height {
            height
        } else {
            let mode_height = output.mode_height?;
            let scale = output.scale.max(1);
            ((mode_height as f64) / (scale as f64)).round() as i32
        };
        Some((x, y, width, height))
    }

    fn geometry_close(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
        fn close(a: i32, b: i32) -> bool {
            (a - b).abs() <= 1
        }

        close(a.0, b.0) && close(a.1, b.1) && close(a.2, b.2) && close(a.3, b.3)
    }

    fn is_missing_screencopy(err: &anyhow::Error) -> bool {
        let msg = err.to_string().to_ascii_lowercase();
        msg.contains("screencopy") || msg.contains("wlr-screencopy")
    }
}

#[cfg(all(target_os = "linux", feature = "freeze"))]
pub use imp::FreezeGuard;
#[cfg(all(target_os = "linux", feature = "freeze"))]
pub use imp::start_freeze;

#[cfg(not(all(target_os = "linux", feature = "freeze")))]
mod imp_stub {
    use super::*;

    pub struct FreezeGuard;

    impl FreezeGuard {
        pub fn stop(self) -> Result<()> {
            Ok(())
        }
    }

    pub fn start_freeze(_selected_output: Option<&str>, _debug: bool) -> Result<FreezeGuard> {
        Ok(FreezeGuard)
    }
}

#[cfg(not(all(target_os = "linux", feature = "freeze")))]
pub use imp_stub::FreezeGuard;
#[cfg(not(all(target_os = "linux", feature = "freeze")))]
pub use imp_stub::start_freeze;
