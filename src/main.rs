#![cfg_attr(all(target_os = "windows", not(test)), windows_subsystem = "windows")]

use anyhow::Result;

#[cfg(not(target_os = "windows"))]
use clap::Parser;

#[cfg(target_os = "windows")]
mod windows_app;
#[cfg(target_os = "windows")]
mod windows_capture;
#[cfg(target_os = "windows")]
mod windows_i18n;
#[cfg(target_os = "windows")]
mod windows_icon;
#[cfg(target_os = "windows")]
mod windows_ui;

#[cfg(not(target_os = "windows"))]
mod app;
#[cfg(not(target_os = "windows"))]
mod capture;
#[cfg(not(target_os = "windows"))]
mod cli;
mod config;
#[cfg(not(target_os = "windows"))]
mod config_cmds;
#[cfg(target_os = "linux")]
mod embedded_slurp;
#[cfg(not(target_os = "windows"))]
mod freeze;
#[cfg(not(target_os = "windows"))]
mod geometry;
#[cfg(not(target_os = "windows"))]
mod hyprland_cmds;
#[cfg(not(target_os = "windows"))]
mod save;
#[cfg(not(target_os = "windows"))]
mod utils;
#[cfg(not(target_os = "windows"))]
pub use cli::{Args, Mode, default_filename, resolve_delay, resolve_notif_timeout};

#[cfg(not(target_os = "windows"))]
fn main() -> Result<()> {
    let args = Args::parse();
    app::run(args)
}

#[cfg(target_os = "windows")]
fn main() -> Result<()> {
    if std::env::args().any(|arg| arg == "--settings-ui") {
        return windows_ui::run_settings();
    }
    windows_app::run()
}

#[cfg(test)]
mod tests;
