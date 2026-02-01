use crate::{Args, Mode, default_filename, resolve_delay, resolve_notif_timeout};
use chrono::TimeZone;
use clap::Parser;
use std::time::Duration;

#[test]
fn parse_output_name_mode() {
    let args = Args::parse_from(["hyprshot-rs", "-m", "output", "-m", "DP-1"]);
    assert!(matches!(args.mode.get(0), Some(Mode::Output)));
    assert!(matches!(
        args.mode.get(1),
        Some(Mode::OutputName(name)) if name == "DP-1"
    ));
}

#[test]
fn notif_timeout_cli_overrides_config() {
    let mut config = crate::config::Config::default();
    config.capture.notification_timeout = 7000;

    let args = Args::parse_from(["hyprshot-rs", "-m", "region", "--notif-timeout", "5000"]);

    assert_eq!(resolve_notif_timeout(&args, &config), 5000);
}

#[test]
fn delay_uses_milliseconds_from_config() {
    let mut config = crate::config::Config::default();
    config.advanced.delay_ms = 250;

    let args = Args::parse_from(["hyprshot-rs", "-m", "region"]);
    assert_eq!(resolve_delay(&args, &config), Duration::from_millis(250));
}

#[test]
fn filename_includes_milliseconds() {
    let now = chrono::Local
        .timestamp_millis_opt(1_700_000_000_123)
        .unwrap();
    let name = default_filename(now);
    assert!(name.ends_with("-123_hyprshot.png"));
}
