use anyhow::{Context, Result};
use serde_json::Value;
use std::io::Read;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::geometry::Geometry;

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

    if let Ok(monitors_output) = output_with_timeout(
        {
            let mut cmd = Command::new("hyprctl");
            cmd.arg("monitors").arg("-j");
            cmd
        },
        Duration::from_secs(3),
    ) {
        if let Ok(monitors) = serde_json::from_slice::<Value>(&monitors_output.stdout) {
            if let Some(monitor) = monitors.as_array().and_then(|arr| {
                arr.iter().find(|m| {
                    let mx = m["x"].as_i64().unwrap_or(0) as i32;
                    let my = m["y"].as_i64().unwrap_or(0) as i32;
                    let mw = m["width"].as_i64().unwrap_or(0) as i32;
                    let mh = m["height"].as_i64().unwrap_or(0) as i32;
                    x >= mx && x < mx + mw && y >= my && y < my + mh
                })
            }) {
                mon_x = monitor["x"].as_i64().unwrap_or(0) as i32;
                mon_y = monitor["y"].as_i64().unwrap_or(0) as i32;
                mon_width = monitor["width"].as_i64().unwrap_or(0) as i32;
                mon_height = monitor["height"].as_i64().unwrap_or(0) as i32;
                found = true;
            }
        }
    }

    if !found {
        if let Ok(outputs_output) = output_with_timeout(
            {
                let mut cmd = Command::new("swaymsg");
                cmd.arg("-t").arg("get_outputs");
                cmd
            },
            Duration::from_secs(3),
        ) {
            if let Ok(outputs) = serde_json::from_slice::<Value>(&outputs_output.stdout) {
                if let Some(output) = outputs.as_array().and_then(|arr| {
                    arr.iter().find(|o| {
                        let rect = o["rect"].as_object();
                        let mx = rect
                            .and_then(|r| r.get("x"))
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0) as i32;
                        let my = rect
                            .and_then(|r| r.get("y"))
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0) as i32;
                        let mw = rect
                            .and_then(|r| r.get("width"))
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0) as i32;
                        let mh = rect
                            .and_then(|r| r.get("height"))
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0) as i32;
                        x >= mx && x < mx + mw && y >= my && y < my + mh
                    })
                }) {
                    let rect = output["rect"]
                        .as_object()
                        .context("Invalid sway output rect")?;
                    mon_x = rect.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    mon_y = rect.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    mon_width = rect.get("width").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    mon_height = rect.get("height").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    found = true;
                }
            }
        }
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
