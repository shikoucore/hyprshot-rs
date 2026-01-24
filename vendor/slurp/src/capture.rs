use anyhow::{Context, Result};
use serde_json::Value;
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[cfg(target_os = "linux")]
use crate::embedded_slurp::get_slurp_path;

#[cfg(not(target_os = "linux"))]
fn get_slurp_path() -> Result<std::path::PathBuf> {
    Ok(std::path::PathBuf::from("slurp"))
}

pub fn grab_output(debug: bool) -> Result<String> {
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
    Ok(geometry)
}

pub fn grab_active_output(debug: bool) -> Result<String> {
    let active_workspace: Value = serde_json::from_slice(
        &Command::new("hyprctl")
            .arg("activeworkspace")
            .arg("-j")
            .output()
            .context("Failed to run hyprctl activeworkspace")?
            .stdout,
    )?;
    let monitors: Value = serde_json::from_slice(
        &Command::new("hyprctl")
            .arg("monitors")
            .arg("-j")
            .output()
            .context("Failed to run hyprctl monitors")?
            .stdout,
    )?;

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

    let x = current_monitor["x"].as_i64().unwrap_or(0);
    let y = current_monitor["y"].as_i64().unwrap_or(0);
    let width = current_monitor["width"].as_i64().unwrap_or(0) as f64;
    let height = current_monitor["height"].as_i64().unwrap_or(0) as f64;
    let scale = current_monitor["scale"].as_f64().unwrap_or(1.0);

    let geometry = format!(
        "{},{} {}x{}",
        x,
        y,
        (width / scale).round() as i32,
        (height / scale).round() as i32
    );
    if debug {
        eprintln!("Active output geometry: {}", geometry);
    }
    Ok(geometry)
}

pub fn grab_selected_output(monitor: &str, debug: bool) -> Result<String> {
    let monitors: Value = serde_json::from_slice(
        &Command::new("hyprctl")
            .arg("monitors")
            .arg("-j")
            .output()
            .context("Failed to run hyprctl monitors")?
            .stdout,
    )?;

    let monitor_data = monitors
        .as_array()
        .and_then(|arr| arr.iter().find(|m| m["name"].as_str() == Some(monitor)))
        .context(format!("Monitor '{}' not found", monitor))?;

    if debug {
        eprintln!("Capturing monitor: {}", monitor);
    }

    let x = monitor_data["x"].as_i64().unwrap_or(0);
    let y = monitor_data["y"].as_i64().unwrap_or(0);
    let width = monitor_data["width"].as_i64().unwrap_or(0) as f64;
    let height = monitor_data["height"].as_i64().unwrap_or(0) as f64;
    let scale = monitor_data["scale"].as_f64().unwrap_or(1.0);

    let geometry = format!(
        "{},{} {}x{}",
        x,
        y,
        (width / scale).round() as i32,
        (height / scale).round() as i32
    );
    if debug {
        eprintln!("Selected output geometry: {}", geometry);
    }
    Ok(geometry)
}

pub fn grab_region(debug: bool) -> Result<String> {
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
    Ok(geometry)
}

pub fn grab_window(debug: bool) -> Result<String> {
    let monitors: Value = serde_json::from_slice(
        &Command::new("hyprctl")
            .arg("monitors")
            .arg("-j")
            .output()
            .context("Failed to run hyprctl monitors")?
            .stdout,
    )?;
    let clients: Value = serde_json::from_slice(
        &Command::new("hyprctl")
            .arg("clients")
            .arg("-j")
            .output()
            .context("Failed to run hyprctl clients")?
            .stdout,
    )?;

    let workspace_ids: String = monitors
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m["activeWorkspace"]["id"].as_i64())
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default();

    let filtered_clients: Vec<Value> = clients
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|c| {
                    c["workspace"]["id"]
                        .as_i64()
                        .map(|id| workspace_ids.contains(&id.to_string()))
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

    let parts: Vec<&str> = geometry.split(' ').collect();
    if parts.len() != 2 || parts[0].split(',').count() != 2 || parts[1].split('x').count() != 2 {
        return Err(anyhow::anyhow!("Invalid geometry format: '{}'", geometry));
    }

    Ok(geometry)
}

pub fn grab_active_window(debug: bool) -> Result<String> {
    let active_window: Value = serde_json::from_slice(
        &Command::new("hyprctl")
            .arg("activewindow")
            .arg("-j")
            .output()
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

    let geometry = format!("{},{} {}x{}", x, y, width, height);
    if debug {
        eprintln!("Active window geometry: {}", geometry);
    }
    Ok(geometry)
}
