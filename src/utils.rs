use anyhow::{Context, Result};
use std::io::Read;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::geometry::Geometry;

#[cfg(feature = "freeze")]
use wayland_client::{
    Connection, Dispatch, QueueHandle,
    protocol::{wl_output::Mode as WlOutputMode, wl_output::WlOutput, wl_registry::WlRegistry},
};
#[cfg(feature = "freeze")]
use wayland_protocols::xdg::xdg_output::zv1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1, zxdg_output_v1::ZxdgOutputV1,
};

pub fn trim(geometry: &Geometry, debug: bool) -> Result<Geometry> {
    if debug {
        eprintln!("Input geometry: {}", geometry);
    }
    let x = geometry.x;
    let y = geometry.y;
    let width = geometry.width;
    let height = geometry.height;

    let mut mon_x = 0;
    let mut mon_y = 0;
    let mut mon_width = 0;
    let mut mon_height = 0;

    let mut found = false;

    #[cfg(feature = "freeze")]
    if let Some((mx, my, mw, mh)) = wayland_monitor_bounds(x, y)? {
        mon_x = mx;
        mon_y = my;
        mon_width = mw;
        mon_height = mh;
        found = true;
    }

    if !found {
        if debug {
            eprintln!("Warning: could not determine monitor bounds; using raw geometry");
        }
        return Ok(*geometry);
    }

    let mut cropped_x = x;
    let mut cropped_y = y;
    let mut cropped_width = width;
    let mut cropped_height = height;

    if x + width > mon_x + mon_width {
        cropped_width = mon_x + mon_width - x;
    }
    if y + height > mon_y + mon_height {
        cropped_height = mon_y + mon_height - y;
    }
    if x < mon_x {
        cropped_x = mon_x;
        cropped_width -= mon_x - x;
    }
    if y < mon_y {
        cropped_y = mon_y;
        cropped_height -= mon_y - y;
    }

    if cropped_width <= 0 || cropped_height <= 0 {
        return Err(anyhow::anyhow!(
            "Invalid cropped dimensions: width={} or height={}",
            cropped_width,
            cropped_height
        ));
    }

    let cropped = Geometry::new(cropped_x, cropped_y, cropped_width, cropped_height)?;
    if debug {
        eprintln!("Cropped geometry: {}", cropped);
    }
    Ok(cropped)
}

#[cfg(feature = "freeze")]
fn wayland_monitor_bounds(x: i32, y: i32) -> Result<Option<(i32, i32, i32, i32)>> {
    let conn = Connection::connect_to_env().context("Failed to connect to Wayland")?;
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let _registry = conn.display().get_registry(&qh, ());

    #[derive(Debug)]
    struct OutputKey(usize);

    struct OutputEntry {
        output: WlOutput,
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

    struct State {
        outputs: Vec<OutputEntry>,
        xdg_output_manager: Option<ZxdgOutputManagerV1>,
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
                _ => {}
            }
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

    let mut state = State {
        outputs: Vec::new(),
        xdg_output_manager: None,
    };

    event_queue
        .roundtrip(&mut state)
        .context("Failed to initialize Wayland outputs")?;

    if let Some(manager) = &state.xdg_output_manager {
        for (idx, entry) in state.outputs.iter_mut().enumerate() {
            let xdg_output = manager.get_xdg_output(&entry.output, &qh, OutputKey(idx));
            entry.xdg_output = Some(xdg_output);
        }
        event_queue
            .roundtrip(&mut state)
            .context("Failed to receive Wayland output geometry")?;
    }

    fn output_logical_size(output: &OutputEntry) -> Option<(i32, i32)> {
        if let (Some(width), Some(height)) = (output.logical_width, output.logical_height) {
            return Some((width, height));
        }
        let mode_width = output.mode_width?;
        let mode_height = output.mode_height?;
        let scale = output.scale.max(1);
        Some((
            ((mode_width as f64) / (scale as f64)).round() as i32,
            ((mode_height as f64) / (scale as f64)).round() as i32,
        ))
    }

    for output in &state.outputs {
        let (ox, oy) = match (
            output.logical_x.or(output.pos_x),
            output.logical_y.or(output.pos_y),
        ) {
            (Some(ox), Some(oy)) => (ox, oy),
            _ => continue,
        };
        let (ow, oh) = match output_logical_size(output) {
            Some(v) => v,
            None => continue,
        };
        if x >= ox && x < ox + ow && y >= oy && y < oy + oh {
            return Ok(Some((ox, oy, ow, oh)));
        }
    }

    Ok(None)
}

// Wait for a spawned process with a hard timeout; used for wl-copy in save.rs.
pub fn wait_with_timeout(child: &mut Child, timeout: Duration) -> Result<ExitStatus> {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().context("Failed to poll process status")? {
            return Ok(status);
        }
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(anyhow::anyhow!("Process timed out after {:?}", timeout));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

// Run a short-lived command with a timeout and capture Output; used for hyprctl/swaymsg.
pub fn output_with_timeout(mut cmd: Command, timeout: Duration) -> Result<Output> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().context("Failed to spawn command")?;
    let mut stdout = child
        .stdout
        .take()
        .context("Failed to capture command stdout")?;
    let mut stderr = child
        .stderr
        .take()
        .context("Failed to capture command stderr")?;
    let out_handle = thread::spawn(move || -> std::io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        stdout.read_to_end(&mut buf)?;
        Ok(buf)
    });
    let err_handle = thread::spawn(move || -> std::io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        stderr.read_to_end(&mut buf)?;
        Ok(buf)
    });
    let status = match wait_with_timeout(&mut child, timeout) {
        Ok(status) => status,
        Err(err) => {
            let _ = out_handle.join();
            let _ = err_handle.join();
            return Err(err);
        }
    };
    let stdout = out_handle
        .join()
        .unwrap_or_else(|_| Ok(Vec::new()))
        .context("Failed to read command stdout")?;
    let stderr = err_handle
        .join()
        .unwrap_or_else(|_| Ok(Vec::new()))
        .context("Failed to read command stderr")?;
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}
