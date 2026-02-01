use anyhow::{Context, Result};

#[cfg(all(target_os = "linux", feature = "freeze"))]
mod imp {
    use super::*;
    use grim_rs::Grim;
    use std::{
        os::fd::{AsRawFd, BorrowedFd},
        sync::mpsc,
        thread,
        time::Duration,
    };
    use wayland_client::{
        Connection, Dispatch, EventQueue, QueueHandle,
        protocol::{
            wl_buffer::WlBuffer,
            wl_callback,
            wl_compositor::WlCompositor,
            wl_output::Mode as WlOutputMode,
            wl_output::WlOutput,
            wl_region::WlRegion,
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
    struct GrimOutputMeta {
        name: String,
        geom: (i32, i32, i32, i32),
    }

    struct CaptureImage {
        data: Vec<u8>,
        width: u32,
        height: u32,
    }

    pub fn start_freeze(selected_output: Option<&str>, debug: bool) -> Result<FreezeGuard> {
        let (stop_tx, stop_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();

        let selected_output = selected_output.map(str::to_string);
        let join = thread::spawn(move || run_freeze(selected_output, stop_rx, ready_tx, debug));

        match ready_rx.recv_timeout(Duration::from_millis(200)) {
            Ok(Ok(())) => Ok(FreezeGuard {
                stop_tx,
                join: Some(join),
            }),
            Ok(Err(err)) => {
                eprintln!("Freeze disabled: {}", err);
                Ok(FreezeGuard {
                    stop_tx,
                    join: None,
                })
            }
            Err(_) => Ok(FreezeGuard {
                stop_tx,
                join: None,
            }),
        }
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
        _input_region: WlRegion,
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
        frame_done: bool,
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

    impl Dispatch<WlRegion, ()> for State {
        fn event(
            _: &mut Self,
            _: &WlRegion,
            _: wayland_client::protocol::wl_region::Event,
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
        selected_output: Option<String>,
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
            frame_done: false,
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
            .context("wl_compositor not available")?
            .clone();
        let shm = state.shm.as_ref().context("wl_shm not available")?.clone();
        let layer_shell = match state.layer_shell.as_ref() {
            Some(shell) => shell.clone(),
            None => {
                // FIXME: нужно проверить поддержку wlr-layer-shell на Hyprland/Sway/River/Wayfire.
                eprintln!(
                    "Freeze is disabled: compositor does not support wlr-layer-shell. \
Check the support for this protocol on Hyprland/Sway/River/Wayfire."
                );
                let _ = ready_tx.send(Ok(()));
                return Ok(());
            }
        };

        // Sync with the next compositor frame before capturing outputs to avoid
        // stale selection UI from a previous run.
        if let Err(err) = sync_next_frame(&mut event_queue, &qh, &compositor, &mut state)
            && debug
        {
            eprintln!("Freeze frame sync failed: {}", err);
        }

        let mut grim = match Grim::new() {
            Ok(grim) => grim,
            Err(err) if is_missing_screencopy_msg(&err.to_string()) => {
                // FIXME: нужно проверить поддержку wlr-screencopy на Hyprland/Sway/River/Wayfire.
                eprintln!(
                    "Freeze is disabled: compositor does not support wlr-screencopy. \
        Check the support for this protocol on Hyprland/Sway/River/Wayfire."
                );
                let _ = ready_tx.send(Ok(()));
                return Ok(());
            }
            Err(err) => {
                let _ = ready_tx.send(Err(err.into()));
                return Ok(());
            }
        };

        if stop_rx.try_recv().is_ok() {
            let _ = ready_tx.send(Ok(()));
            return Ok(());
        }

        let grim_outputs = grim
            .get_outputs()
            .context("Failed to list outputs via grim-rs")?;
        let mut metas = Vec::new();
        for output in grim_outputs {
            metas.push(GrimOutputMeta {
                name: output.name().to_string(),
                geom: (
                    output.geometry().x(),
                    output.geometry().y(),
                    output.geometry().width(),
                    output.geometry().height(),
                ),
            });
        }

        let mapping = match_outputs(&state.outputs, &metas, selected_output.as_deref())?;
        if mapping.iter().all(|m| m.is_none()) {
            let _ = ready_tx.send(Err(anyhow::anyhow!(
                "No matching outputs found for freeze overlay"
            )));
            return Ok(());
        }

        for (idx, meta_index) in mapping.into_iter().enumerate() {
            if stop_rx.try_recv().is_ok() {
                let _ = ready_tx.send(Ok(()));
                return Ok(());
            }
            let Some(meta_index) = meta_index else {
                continue;
            };
            let output = &state.outputs[idx];
            let meta = &metas[meta_index];

            let capture = grim
                .capture_output(&meta.name)
                .with_context(|| format!("Failed to capture output '{}'", meta.name))?;

            if debug {
                eprintln!(
                    "Freeze capture: {} ({}x{})",
                    meta.name,
                    capture.width(),
                    capture.height()
                );
            }

            let width = capture.width();
            let height = capture.height();
            let capture = CaptureImage {
                data: capture.into_data(),
                width,
                height,
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

            if let Some((logical_w, logical_h)) = output_logical_size(output)
                && logical_w > 0
                && logical_h > 0
            {
                layer_surface.set_size(logical_w as u32, logical_h as u32);
            }

            let buffer_scale = output_buffer_scale(output);
            if buffer_scale > 1 {
                surface.set_buffer_scale(buffer_scale);
            }

            let input_region = compositor.create_region(&qh, ());
            surface.set_input_region(Some(&input_region));

            surface.commit();

            let (buffer, tmp, mmap) = create_buffer(&shm, &qh, &capture).with_context(|| {
                format!(
                    "Failed to create buffer for output '{}'",
                    output.name.as_deref().unwrap_or(&meta.name)
                )
            })?;

            state.surfaces.push(SurfaceEntry {
                surface,
                layer_surface,
                buffer,
                _input_region: input_region,
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

    struct FrameSync;

    impl Dispatch<wl_callback::WlCallback, FrameSync> for State {
        fn event(
            state: &mut Self,
            callback: &wl_callback::WlCallback,
            event: wl_callback::Event,
            _: &FrameSync,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            if let wl_callback::Event::Done { .. } = event {
                state.frame_done = true;
                let _ = callback;
            }
        }
    }

    fn sync_next_frame(
        event_queue: &mut EventQueue<State>,
        qh: &QueueHandle<State>,
        compositor: &WlCompositor,
        state: &mut State,
    ) -> Result<()> {
        let surface = compositor.create_surface(qh, ());
        state.frame_done = false;
        surface.frame(qh, FrameSync);
        surface.commit();

        while !state.frame_done {
            event_queue
                .roundtrip(state)
                .context("Failed to sync frame")?;
        }

        surface.destroy();
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

    fn output_geometry(output: &OutputEntry) -> Option<(i32, i32, i32, i32)> {
        let x = output.logical_x.or(output.pos_x)?;
        let y = output.logical_y.or(output.pos_y)?;
        let (width, height) = output_logical_size(output)?;
        Some((x, y, width, height))
    }

    fn geometry_close(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
        fn close(a: i32, b: i32) -> bool {
            (a - b).abs() <= 1
        }

        close(a.0, b.0) && close(a.1, b.1) && close(a.2, b.2) && close(a.3, b.3)
    }

    fn output_buffer_scale(output: &OutputEntry) -> i32 {
        if let (Some(mode_width), Some(logical_width)) = (output.mode_width, output.logical_width)
            && logical_width > 0
        {
            let scale = (mode_width as f64) / (logical_width as f64);
            if (scale - scale.round()).abs() < 0.01 {
                return scale.round().max(1.0) as i32;
            }
            return 1;
        }
        output.scale.max(1)
    }

    fn match_outputs(
        outputs: &[OutputEntry],
        metas: &[GrimOutputMeta],
        selected_output: Option<&str>,
    ) -> Result<Vec<Option<usize>>> {
        let mut mapping = vec![None; outputs.len()];
        let mut used = vec![false; metas.len()];

        if let Some(selected) = selected_output {
            let meta_index = metas
                .iter()
                .position(|meta| meta.name == selected)
                .context(format!("Output '{}' not found", selected))?;

            if let Some((idx, _)) = outputs
                .iter()
                .enumerate()
                .find(|(_, o)| o.name.as_deref() == Some(selected))
            {
                mapping[idx] = Some(meta_index);
                used[meta_index] = true;
                return Ok(mapping);
            }

            let target_geom = metas[meta_index].geom;
            if let Some((idx, _)) = outputs.iter().enumerate().find(|(_, o)| {
                output_geometry(o)
                    .map(|geom| geometry_close(geom, target_geom))
                    .unwrap_or(false)
            }) {
                mapping[idx] = Some(meta_index);
                used[meta_index] = true;
                return Ok(mapping);
            }

            if !outputs.is_empty() {
                mapping[0] = Some(meta_index);
                used[meta_index] = true;
            }

            return Ok(mapping);
        }

        for (idx, output) in outputs.iter().enumerate() {
            let Some(name) = output.name.as_deref() else {
                continue;
            };
            if let Some((meta_idx, _)) = metas
                .iter()
                .enumerate()
                .find(|(m_idx, meta)| !used[*m_idx] && meta.name == name)
            {
                mapping[idx] = Some(meta_idx);
                used[meta_idx] = true;
            }
        }

        for (idx, output) in outputs.iter().enumerate() {
            if mapping[idx].is_some() {
                continue;
            }
            let Some(geom) = output_geometry(output) else {
                continue;
            };
            if let Some((meta_idx, _)) = metas
                .iter()
                .enumerate()
                .find(|(m_idx, meta)| !used[*m_idx] && geometry_close(meta.geom, geom))
            {
                mapping[idx] = Some(meta_idx);
                used[meta_idx] = true;
            }
        }

        let mut unused = metas
            .iter()
            .enumerate()
            .filter(|(idx, _)| !used[*idx])
            .map(|(idx, _)| idx);

        for slot in mapping.iter_mut().take(outputs.len()) {
            if slot.is_none()
                && let Some(meta_idx) = unused.next()
            {
                *slot = Some(meta_idx);
            }
        }

        Ok(mapping)
    }

    fn is_missing_screencopy_msg(msg: &str) -> bool {
        let msg = msg.to_ascii_lowercase();
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
