use anyhow::Result;
use clap::Parser;

mod app;
mod capture;
mod cli;
mod config;
mod config_cmds;
mod freeze;
mod geometry;
mod hyprland_cmds;
mod save;
mod selector;
mod utils;
pub use cli::{Args, Mode, default_filename, resolve_delay, resolve_notif_timeout};

fn main() -> Result<()> {
    let args = Args::parse();
    app::run(args)
}
#[cfg(test)]
mod tests;
