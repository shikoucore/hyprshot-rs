use anyhow::{Context, Result};
use notify_rust::Notification;
use std::fs::{create_dir_all, write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::geometry::Geometry;

#[cfg(feature = "grim")]
#[allow(clippy::too_many_arguments)]
pub fn save_geometry_with_grim(
    geometry: &Geometry,
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

    let region: grim_rs::Box = geometry
        .to_string()
        .parse()
        .context("Failed to parse geometry into grim-rs::Box")?;

    let mut grim = grim_rs::Grim::new().context("Failed to initialize grim-rs")?;

    let capture_result = grim
        .capture_region(region)
        .context("Failed to capture screenshot region")?;

    let png_bytes = grim
        .to_png(
            capture_result.data(),
            capture_result.width(),
            capture_result.height(),
        )
        .context("Failed to encode screenshot as PNG")?;

    if raw {
        std::io::stdout().write_all(&png_bytes)?;
        return Ok(());
    }

    if !clipboard_only {
        create_dir_all(save_fullpath.parent().unwrap())
            .context("Failed to create screenshot directory")?;

        write(save_fullpath, &png_bytes).context(format!(
            "Failed to save screenshot to '{}'",
            save_fullpath.display()
        ))?;

        let wl_copy_result = (|| -> Result<()> {
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
            let status = wl_copy.wait().context("Failed to wait for wl-copy")?;
            if !status.success() {
                return Err(anyhow::anyhow!("wl-copy failed to copy screenshot"));
            }
            Ok(())
        })();
        if let Err(err) = wl_copy_result {
            eprintln!("Warning: failed to copy screenshot to clipboard: {}", err);
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
        if let Err(err) = Notification::new()
            .summary("Screenshot saved")
            .body(&message)
            .icon(save_fullpath.to_str().unwrap_or("screenshot"))
            .timeout(notif_timeout as i32)
            .appname("Hyprshot-rs")
            .show()
        {
            eprintln!("Warning: failed to show notification: {}", err);
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn save_geometry(
    geometry: &Geometry,
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
    #[cfg(not(feature = "grim"))]
    compile_error!("Feature 'grim' must be enabled to save screenshots");
}
