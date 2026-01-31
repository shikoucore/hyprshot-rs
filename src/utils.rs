use anyhow::{Context, Result};
use serde_json::Value;
use std::process::Command;

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

    if let Ok(monitors_output) = Command::new("hyprctl").arg("monitors").arg("-j").output() {
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
        if let Ok(outputs_output) = Command::new("swaymsg")
            .arg("-t")
            .arg("get_outputs")
            .output()
        {
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
