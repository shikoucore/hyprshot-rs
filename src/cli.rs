use chrono::{DateTime, Local};
use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;

use crate::config;

#[derive(Parser)]
#[command(
    name = "hyprshot-rs",
    about = "Utility to easily take screenshots in Hyprland"
)]
pub struct Args {
    #[arg(
        short = 'm',
        long,
        value_parser = clap::builder::ValueParser::new(
            |s: &str| -> std::result::Result<Mode, String> {
            match s.to_ascii_lowercase().as_str() {
                "output" => Ok(Mode::Output),
                "window" => Ok(Mode::Window),
                "region" => Ok(Mode::Region),
                "active" => Ok(Mode::Active),
                _ => Ok(Mode::OutputName(s.to_string())),
            }
        }),
        help = "Mode: output, window, region, active, or OUTPUT_NAME"
    )]
    pub mode: Vec<Mode>,

    #[arg(short, long, help = "Directory to save screenshot")]
    pub output_folder: Option<PathBuf>,

    #[arg(short, long, help = "Filename of the screenshot")]
    pub filename: Option<String>,

    #[arg(short = 'D', long, help = "Delay before taking screenshot (seconds)")]
    pub delay: Option<u64>,

    #[arg(long, help = "Freeze the screen on initialization")]
    pub freeze: bool,

    #[arg(short, long, help = "Print debug information")]
    pub debug: bool,

    #[arg(short, long, help = "Don't send notification")]
    pub silent: bool,

    #[arg(short, long, help = "Output raw image data to stdout")]
    pub raw: bool,

    #[arg(short, long, help = "Notification timeout (ms)")]
    pub notif_timeout: Option<u32>,

    #[arg(long, help = "Copy to clipboard and don't save to disk")]
    pub clipboard_only: bool,

    #[arg(last = true, help = "Command to open screenshot (e.g., 'mirage')")]
    pub command: Vec<String>,

    // Config management commands
    #[arg(long, help = "Initialize default config file")]
    pub init_config: bool,

    #[arg(long, help = "Show current configuration")]
    pub show_config: bool,

    #[arg(long, help = "Show path to config file")]
    pub config_path: bool,

    #[arg(
        long,
        value_names = ["KEY", "VALUE"],
        num_args = 2,
        help = "Set config value (e.g., --set paths.screenshots_dir ~/Screenshots)"
    )]
    pub set: Option<Vec<String>>,

    // Hyprland integration commands
    #[arg(long, help = "Generate Hyprland keybindings")]
    pub generate_hyprland_config: bool,

    #[arg(long, help = "Install keybindings to hyprland.conf (creates backup)")]
    pub install_binds: bool,

    #[arg(long, help = "Include clipboard-only bindings when generating")]
    pub with_clipboard: bool,

    #[arg(long, help = "Interactive hotkeys setup wizard")]
    pub setup_hotkeys: bool,

    #[arg(
        long,
        help = "Don't load configuration file (use defaults and CLI args only)"
    )]
    pub no_config: bool,
}

impl std::fmt::Debug for Args {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Args")
            .field("mode", &self.mode)
            .field("output_folder", &self.output_folder)
            .field("filename", &self.filename)
            .field("delay", &self.delay)
            .field("freeze", &self.freeze)
            .field("debug", &self.debug)
            .field("silent", &self.silent)
            .field("raw", &self.raw)
            .field("notif_timeout", &self.notif_timeout)
            .field("clipboard_only", &self.clipboard_only)
            .field("command", &self.command)
            .finish()
    }
}

pub fn resolve_notif_timeout(args: &Args, config: &config::Config) -> u32 {
    args.notif_timeout
        .unwrap_or(config.capture.notification_timeout)
}

pub fn resolve_delay(args: &Args, config: &config::Config) -> Duration {
    if let Some(d) = args.delay {
        Duration::from_secs(d)
    } else if config.advanced.delay_ms > 0 {
        Duration::from_millis(config.advanced.delay_ms as u64)
    } else {
        Duration::from_secs(0)
    }
}

pub fn default_filename(now: DateTime<Local>) -> String {
    format!(
        "{}-{:03}_hyprshot.png",
        now.format("%Y-%m-%d-%H%M%S"),
        now.timestamp_subsec_millis()
    )
}

#[derive(Clone, Debug)]
pub enum Mode {
    Output,
    Window,
    Region,
    Active,
    OutputName(String),
}
