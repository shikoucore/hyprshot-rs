use anyhow::{Context, Result};
use serde_json::Value;
use std::{
    collections::HashSet,
    io::Write,
    process::{Command, Stdio},
    time::Duration,
};

use crate::geometry::Geometry;
use crate::utils::output_with_timeout;

#[cfg(feature = "freeze")]
use wayland_client::{
    Connection, Dispatch, QueueHandle,
    protocol::{wl_output::Mode as WlOutputMode, wl_output::WlOutput, wl_registry::WlRegistry},
};
#[cfg(feature = "freeze")]
use wayland_protocols::xdg::xdg_output::zv1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1, zxdg_output_v1::ZxdgOutputV1,
};

#[cfg(target_os = "linux")]
use crate::embedded_slurp::get_slurp_path;

#[cfg(not(target_os = "linux"))]
fn get_slurp_path() -> Result<std::path::PathBuf> {
    Ok(std::path::PathBuf::from("slurp"))
}

pub struct HyprctlCache {
    monitors: Option<Value>,
}

impl HyprctlCache {
    pub fn new() -> Self {
        Self { monitors: None }
    }
}

fn hyprctl_monitors_json<'a>(
    cache: &'a mut HyprctlCache,
    timeout: Duration,
) -> Result<&'a Value> {
    if cache.monitors.is_none() {
        let output = output_with_timeout(
            {
                let mut cmd = Command::new("hyprctl");
                cmd.arg("monitors").arg("-j");
                cmd
            },
            timeout,
        )
        .context("Failed to run hyprctl monitors")?;
        let monitors: Value =
            serde_json::from_slice(&output.stdout).context("Failed to parse hyprctl monitors")?;
        cache.monitors = Some(monitors);
    }

    Ok(cache
        .monitors
        .as_ref()
        .context("Hyprctl monitors cache missing")?)
}

pub fn grab_output(debug: bool) -> Result<Geometry> {
    let slurp_path = get_slurp_path()?;

    let output = Command::new(slurp_path)
        .arg("-or")
        .output()
        .context("Failed to run slurp")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("slurp failed to select output"));
    }
    let geometry = String::from_utf8(output.stdout)
        .context("slurp output is not valid UTF-8")?
        .trim()
        .to_string();
    if debug {
        eprintln!("Output geometry: {}", geometry);
    }
    if geometry.is_empty() {
        return Err(anyhow::anyhow!("slurp returned empty geometry"));
    }
    geometry.parse()
}

// Support matrix:
// - region/output: Wayland-wide via slurp
// - output by name: Wayland enumeration (no hyprctl)
// - window/active: Hyprland and Sway (hyprctl/swaymsg)
pub fn grab_active_output(debug: bool, cache: &mut HyprctlCache) -> Result<Geometry> {
    if let Ok(geometry) = grab_active_output_hyprctl(debug, cache) {
        return Ok(geometry);
    }
    if let Ok(geometry) = grab_active_output_sway(debug) {
        return Ok(geometry);
    }

    Err(anyhow::anyhow!(
        "Active output is only supported on Hyprland or Sway"
    ))
}

fn grab_active_output_hyprctl(debug: bool, cache: &mut HyprctlCache) -> Result<Geometry> {
    const IPC_TIMEOUT: Duration = Duration::from_secs(3);
    let active_workspace: Value = serde_json::from_slice(
        &output_with_timeout(
            {
                let mut cmd = Command::new("hyprctl");
                cmd.arg("activeworkspace").arg("-j");
                cmd
            },
            IPC_TIMEOUT,
        )
        .context("Failed to run hyprctl activeworkspace")?
        .stdout,
    )?;
    let monitors = hyprctl_monitors_json(cache, IPC_TIMEOUT)?;

    if debug {
        eprintln!("Monitors: {}", monitors);
        eprintln!("Active workspace: {}", active_workspace);
    }

    let current_monitor = monitors
        .as_array()
        .and_then(|arr| {
            arr.iter()
                .find(|m| m["activeWorkspace"]["id"] == active_workspace["id"])
        })
        .context("No matching monitor found")?;

    if debug {
        eprintln!("Current output: {}", current_monitor);
    }

    let x = current_monitor["x"].as_i64().unwrap_or(0) as i32;
    let y = current_monitor["y"].as_i64().unwrap_or(0) as i32;
    let width = current_monitor["width"].as_i64().unwrap_or(0) as f64;
    let height = current_monitor["height"].as_i64().unwrap_or(0) as f64;
    let scale = current_monitor["scale"].as_f64().unwrap_or(1.0);

    let geometry = Geometry::new(
        x,
        y,
        (width / scale).round() as i32,
        (height / scale).round() as i32,
    )?;
    if debug {
        eprintln!("Active output geometry: {}", geometry);
    }
    Ok(geometry)
}

fn grab_active_output_sway(debug: bool) -> Result<Geometry> {
    let workspaces = sway_msg(&["-t", "get_workspaces"])?;
    let focused_output = workspaces
        .as_array()
        .and_then(|arr| arr.iter().find(|w| w["focused"].as_bool() == Some(true)))
        .and_then(|w| w["output"].as_str())
        .context("Failed to find focused workspace output")?;

    let outputs = sway_msg(&["-t", "get_outputs"])?;
    let output_data = outputs
        .as_array()
        .and_then(|arr| {
            arr.iter()
                .find(|o| o["name"].as_str() == Some(focused_output))
        })
        .context("Focused output not found in sway outputs")?;

    let rect = output_data["rect"]
        .as_object()
        .context("Invalid output rect data")?;

    let x = rect.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
    let y = rect.get("y").and_then(|v| v.as_i64()).unwrap_or(0);
    let width = rect.get("width").and_then(|v| v.as_i64()).unwrap_or(0);
    let height = rect.get("height").and_then(|v| v.as_i64()).unwrap_or(0);

    let geometry = Geometry::new(x as i32, y as i32, width as i32, height as i32)?;
    if debug {
        eprintln!("Active output geometry (sway): {}", geometry);
    }
    Ok(geometry)
}

pub fn grab_selected_output(monitor: &str, debug: bool) -> Result<Geometry> {
    #[cfg(feature = "freeze")]
    if let Ok(geometry) = grab_selected_output_wayland(monitor, debug) {
        return Ok(geometry);
    }

    Err(anyhow::anyhow!(
        "Output '{}' not found via Wayland. Use '-m output' to select interactively.",
        monitor
    ))
}

#[cfg(feature = "freeze")]
fn grab_selected_output_wayland(monitor: &str, debug: bool) -> Result<Geometry> {
    let conn = Connection::connect_to_env().context("Failed to connect to Wayland")?;
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let _registry = conn.display().get_registry(&qh, ());

    #[derive(Debug)]
    struct OutputKey(usize);

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
            .context("Failed to receive output names")?;
    }

    let Some(output) = state
        .outputs
        .iter()
        .find(|o| o.name.as_deref() == Some(monitor))
    else {
        return Err(anyhow::anyhow!(
            "Output names are unavailable or '{}' was not found",
            monitor
        ));
    };

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

    fn output_geometry(output: &OutputEntry) -> Option<Geometry> {
        let x = output.logical_x.or(output.pos_x)?;
        let y = output.logical_y.or(output.pos_y)?;
        let (width, height) = output_logical_size(output)?;
        Geometry::new(x, y, width, height).ok()
    }

    let geometry = output_geometry(output).context("Output geometry not available")?;
    if debug {
        eprintln!("Selected output geometry: {}", geometry);
    }
    Ok(geometry)
}

pub fn grab_region(debug: bool) -> Result<Geometry> {
    let slurp_path = get_slurp_path()?;

    let output = Command::new(slurp_path)
        .arg("-d")
        .output()
        .context("Failed to run slurp")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("slurp failed to select region"));
    }
    let geometry = String::from_utf8(output.stdout)
        .context("slurp output is not valid UTF-8")?
        .trim()
        .to_string();
    if debug {
        eprintln!("Region geometry: {}", geometry);
    }
    if geometry.is_empty() {
        return Err(anyhow::anyhow!("slurp returned empty geometry"));
    }
    geometry.parse()
}

pub fn grab_window(debug: bool, cache: &mut HyprctlCache) -> Result<Geometry> {
    if let Ok(geometry) = grab_window_hyprctl(debug, cache) {
        return Ok(geometry);
    }
    if let Ok(geometry) = grab_window_sway(debug) {
        return Ok(geometry);
    }

    Err(anyhow::anyhow!(
        "Window selection is only supported on Hyprland or Sway"
    ))
}

fn grab_window_hyprctl(debug: bool, cache: &mut HyprctlCache) -> Result<Geometry> {
    const IPC_TIMEOUT: Duration = Duration::from_secs(3);
    let monitors = hyprctl_monitors_json(cache, IPC_TIMEOUT)?;
    let clients: Value = serde_json::from_slice(
        &output_with_timeout(
            {
                let mut cmd = Command::new("hyprctl");
                cmd.arg("clients").arg("-j");
                cmd
            },
            IPC_TIMEOUT,
        )
        .context("Failed to run hyprctl clients")?
        .stdout,
    )?;

    // Use exact workspace ID matching to avoid substring collisions (e.g., "2" vs "12").
    let workspace_ids: HashSet<i64> = monitors
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m["activeWorkspace"]["id"].as_i64())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();

    let filtered_clients: Vec<Value> = clients
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|c| {
                    c["workspace"]["id"]
                        .as_i64()
                        .map(|id| workspace_ids.contains(&id))
                        .unwrap_or(false)
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    if debug {
        eprintln!("Monitors: {}", monitors);
        eprintln!("Clients: {}", serde_json::to_string(&filtered_clients)?);
    }

    let boxes: String = filtered_clients
        .into_iter()
        .filter_map(|c| {
            let at = c["at"].as_array()?;
            let size = c["size"].as_array()?;
            let x = at[0].as_i64()?;
            let y = at[1].as_i64()?;
            let width = size[0].as_i64()?;
            let height = size[1].as_i64()?;
            if width <= 0 || height <= 0 {
                return None;
            }
            Some(format!(
                "{},{} {}x{} {}",
                x,
                y,
                width,
                height,
                c["title"].as_str().unwrap_or("")
            ))
        })
        .collect::<Vec<_>>()
        .join("\n");

    if debug {
        eprintln!("Window boxes:\n{}", boxes);
    }

    if boxes.is_empty() {
        return Err(anyhow::anyhow!("No valid windows found to capture"));
    }

    let slurp_path = get_slurp_path()?;

    let mut slurp = Command::new(slurp_path)
        .arg("-r")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to start slurp")?;

    slurp
        .stdin
        .as_mut()
        .unwrap()
        .write_all(boxes.as_bytes())
        .context("Failed to write to slurp stdin")?;

    let output = slurp.wait_with_output().context("Failed to run slurp")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "slurp failed to select window: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let geometry = String::from_utf8(output.stdout)
        .context("slurp output is not valid UTF-8")?
        .trim()
        .to_string();
    if debug {
        eprintln!("Window geometry: {}", geometry);
    }
    if geometry.is_empty() {
        return Err(anyhow::anyhow!("slurp returned empty geometry"));
    }
    geometry.parse()
}

pub fn grab_active_window(debug: bool) -> Result<Geometry> {
    if let Ok(geometry) = grab_active_window_hyprctl(debug) {
        return Ok(geometry);
    }
    if let Ok(geometry) = grab_active_window_sway(debug) {
        return Ok(geometry);
    }

    Err(anyhow::anyhow!(
        "Active window is only supported on Hyprland or Sway"
    ))
}

fn grab_active_window_hyprctl(debug: bool) -> Result<Geometry> {
    const IPC_TIMEOUT: Duration = Duration::from_secs(3);
    let active_window: Value = serde_json::from_slice(
        &output_with_timeout(
            {
                let mut cmd = Command::new("hyprctl");
                cmd.arg("activewindow").arg("-j");
                cmd
            },
            IPC_TIMEOUT,
        )
        .context("Failed to run hyprctl activewindow")?
        .stdout,
    )?;

    if debug {
        eprintln!("Active window: {}", active_window);
    }

    let at = active_window["at"]
        .as_array()
        .context("Invalid active window data: missing 'at' field")?;
    let size = active_window["size"]
        .as_array()
        .context("Invalid active window data: missing 'size' field")?;

    let x = at[0].as_i64().context("Invalid x coordinate")?;
    let y = at[1].as_i64().context("Invalid y coordinate")?;
    let width = size[0].as_i64().context("Invalid width")?;
    let height = size[1].as_i64().context("Invalid height")?;

    if width <= 0 || height <= 0 {
        return Err(anyhow::anyhow!(
            "Invalid window dimensions: width={} or height={}",
            width,
            height
        ));
    }

    let geometry = Geometry::new(x as i32, y as i32, width as i32, height as i32)?;
    if debug {
        eprintln!("Active window geometry: {}", geometry);
    }
    Ok(geometry)
}

fn grab_window_sway(debug: bool) -> Result<Geometry> {
    let workspaces = sway_msg(&["-t", "get_workspaces"])?;
    let visible_workspaces: HashSet<String> = workspaces
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|w| w["visible"].as_bool() == Some(true))
                .filter_map(|w| w["name"].as_str().map(|s| s.to_string()))
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();

    let tree = sway_msg(&["-t", "get_tree"])?;
    let mut boxes = Vec::new();
    collect_visible_windows(&tree, &visible_workspaces, false, &mut boxes);

    if debug {
        eprintln!("Sway window boxes:\n{}", boxes.join("\n"));
    }

    if boxes.is_empty() {
        return Err(anyhow::anyhow!("No valid windows found to capture (sway)"));
    }

    let slurp_path = get_slurp_path()?;
    let mut slurp = Command::new(slurp_path)
        .arg("-r")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to start slurp")?;

    slurp
        .stdin
        .as_mut()
        .unwrap()
        .write_all(boxes.join("\n").as_bytes())
        .context("Failed to write to slurp stdin")?;

    let output = slurp.wait_with_output().context("Failed to run slurp")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "slurp failed to select window: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let geometry = String::from_utf8(output.stdout)
        .context("slurp output is not valid UTF-8")?
        .trim()
        .to_string();
    if geometry.is_empty() {
        return Err(anyhow::anyhow!("slurp returned empty geometry"));
    }

    geometry.parse()
}

fn grab_active_window_sway(debug: bool) -> Result<Geometry> {
    let tree = sway_msg(&["-t", "get_tree"])?;
    let focused = find_focused_window(&tree).context("Focused window not found (sway)")?;

    let rect = focused["rect"]
        .as_object()
        .context("Invalid focused window rect")?;
    let x = rect.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
    let y = rect.get("y").and_then(|v| v.as_i64()).unwrap_or(0);
    let width = rect.get("width").and_then(|v| v.as_i64()).unwrap_or(0);
    let height = rect.get("height").and_then(|v| v.as_i64()).unwrap_or(0);

    if width <= 0 || height <= 0 {
        return Err(anyhow::anyhow!(
            "Invalid focused window dimensions: width={} height={}",
            width,
            height
        ));
    }

    let geometry = Geometry::new(x as i32, y as i32, width as i32, height as i32)?;
    if debug {
        eprintln!("Active window geometry (sway): {}", geometry);
    }
    Ok(geometry)
}

fn collect_visible_windows(
    node: &Value,
    visible_workspaces: &HashSet<String>,
    mut visible: bool,
    boxes: &mut Vec<String>,
) {
    if node["type"].as_str() == Some("workspace") {
        visible = node
            .get("name")
            .and_then(|v| v.as_str())
            .map(|name| visible_workspaces.contains(name))
            .unwrap_or(false);
    }

    if visible && is_window_node(node) {
        if let Some(line) = format_window_box(node) {
            boxes.push(line);
        }
    }

    if let Some(nodes) = node.get("nodes").and_then(|v| v.as_array()) {
        for child in nodes {
            collect_visible_windows(child, visible_workspaces, visible, boxes);
        }
    }
    if let Some(nodes) = node.get("floating_nodes").and_then(|v| v.as_array()) {
        for child in nodes {
            collect_visible_windows(child, visible_workspaces, visible, boxes);
        }
    }
}

fn is_window_node(node: &Value) -> bool {
    if node["type"].as_str() != Some("con") {
        return false;
    }
    let has_app = node["app_id"].is_string();
    let has_props = node
        .get("window_properties")
        .map(|v| v.is_object())
        .unwrap_or(false);
    has_app || has_props
}

fn format_window_box(node: &Value) -> Option<String> {
    let rect = node.get("rect")?.as_object()?;
    let x = rect.get("x")?.as_i64()? as i32;
    let y = rect.get("y")?.as_i64()? as i32;
    let width = rect.get("width")?.as_i64()? as i32;
    let height = rect.get("height")?.as_i64()? as i32;
    if width <= 0 || height <= 0 {
        return None;
    }
    let title = node
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .replace('\n', " ");
    Some(format!("{},{} {}x{} {}", x, y, width, height, title))
}

fn find_focused_window<'a>(node: &'a Value) -> Option<&'a Value> {
    if node.get("focused").and_then(|v| v.as_bool()) == Some(true) && is_window_node(node) {
        return Some(node);
    }

    if let Some(nodes) = node.get("nodes").and_then(|v| v.as_array()) {
        for child in nodes {
            if let Some(found) = find_focused_window(child) {
                return Some(found);
            }
        }
    }
    if let Some(nodes) = node.get("floating_nodes").and_then(|v| v.as_array()) {
        for child in nodes {
            if let Some(found) = find_focused_window(child) {
                return Some(found);
            }
        }
    }

    None
}

fn sway_msg(args: &[&str]) -> Result<Value> {
    const IPC_TIMEOUT: Duration = Duration::from_secs(3);
    let output = output_with_timeout(
        {
            let mut cmd = Command::new("swaymsg");
            cmd.args(args);
            cmd
        },
        IPC_TIMEOUT,
    )
    .context("Failed to run swaymsg")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "swaymsg failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    serde_json::from_slice(&output.stdout).context("Failed to parse swaymsg JSON")
}
