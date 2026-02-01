use crate::{Args, Mode, default_filename, resolve_delay, resolve_notif_timeout};
use chrono::TimeZone;
use clap::Parser;
use std::time::Duration;
use std::str::FromStr;
use std::{env, path::PathBuf};

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
    let now = match chrono::Local.timestamp_millis_opt(1_700_000_000_123).single() {
        Some(v) => v,
        None => panic!("Failed to construct timestamp for test"),
    };
    let name = default_filename(now);
    assert!(name.ends_with("-123_hyprshot.png"));
}

#[test]
fn filenames_differ_for_distinct_timestamps() {
    let first = match chrono::Local.timestamp_millis_opt(1_700_000_000_001).single() {
        Some(v) => v,
        None => panic!("Failed to construct first timestamp for test"),
    };
    let second = match chrono::Local.timestamp_millis_opt(1_700_000_000_002).single() {
        Some(v) => v,
        None => panic!("Failed to construct second timestamp for test"),
    };
    let a = default_filename(first);
    let b = default_filename(second);
    assert_ne!(a, b);
}

#[test]
fn geometry_parses_and_validates() {
    let geometry = match crate::geometry::Geometry::from_str("10,20 300x400") {
        Ok(v) => v,
        Err(err) => panic!("Failed to parse geometry: {}", err),
    };
    assert_eq!(geometry.x, 10);
    assert_eq!(geometry.y, 20);
    assert_eq!(geometry.width, 300);
    assert_eq!(geometry.height, 400);

    assert!(crate::geometry::Geometry::from_str("10,20 0x400").is_err());
    assert!(crate::geometry::Geometry::from_str("10,20 -1x400").is_err());
    assert!(crate::geometry::Geometry::from_str("10,20 300x0").is_err());
}

#[test]
fn parse_active_output_mode_combo() {
    let args = Args::parse_from(["hyprshot-rs", "-m", "output", "-m", "active"]);
    assert!(matches!(args.mode.get(0), Some(Mode::Output)));
    assert!(matches!(args.mode.get(1), Some(Mode::Active)));
}

#[test]
fn test_default_config() {
    let config = crate::config::Config::default();
    assert_eq!(config.paths.screenshots_dir, "~/Pictures");
    assert_eq!(config.hotkeys.window, "SUPER, Print");
    assert!(config.capture.notification);
    assert_eq!(config.capture.notification_timeout, 3000);
    assert!(config.advanced.freeze_on_region);
    assert_eq!(config.advanced.delay_ms, 0);
}

#[test]
fn test_config_serialization() {
    let config = crate::config::Config::default();
    let toml_str = match toml::to_string(&config) {
        Ok(v) => v,
        Err(err) => panic!("Failed to serialize config: {}", err),
    };
    assert!(toml_str.contains("[paths]"));
    assert!(toml_str.contains("[hotkeys]"));
    assert!(toml_str.contains("[capture]"));
    assert!(toml_str.contains("[advanced]"));
}

#[test]
fn test_config_deserialization() {
    let toml_str = r#"
        [paths]
        screenshots_dir = "~/Documents"

        [hotkeys]
        window = "ALT, W"
        region = "ALT, R"

        [capture]
        notification = false

        [advanced]
        delay_ms = 500
    "#;

    let config: crate::config::Config = match toml::from_str(toml_str) {
        Ok(v) => v,
        Err(err) => panic!("Failed to deserialize config: {}", err),
    };
    assert_eq!(config.paths.screenshots_dir, "~/Documents");
    assert_eq!(config.hotkeys.window, "ALT, W");
    assert_eq!(config.capture.notification, false);
    assert_eq!(config.advanced.delay_ms, 500);
}

#[test]
fn test_partial_config() {
    let toml_str = r#"
        [paths]
        screenshots_dir = "~/Custom"
    "#;

    let config: crate::config::Config = match toml::from_str(toml_str) {
        Ok(v) => v,
        Err(err) => panic!("Failed to deserialize partial config: {}", err),
    };
    assert_eq!(config.paths.screenshots_dir, "~/Custom");
    assert_eq!(config.hotkeys.window, "SUPER, Print");
    assert!(config.capture.notification);
}

#[test]
fn test_expand_path_tilde() {
    let result = match crate::config::expand_path("~/Pictures") {
        Ok(v) => v,
        Err(err) => panic!("Failed to expand path: {}", err),
    };
    let home = match dirs::home_dir() {
        Some(v) => v,
        None => panic!("Failed to resolve home directory"),
    };
    assert_eq!(result, home.join("Pictures"));

    let result = match crate::config::expand_path("~") {
        Ok(v) => v,
        Err(err) => panic!("Failed to expand home path: {}", err),
    };
    assert_eq!(result, home);
}

#[test]
fn test_expand_path_env_vars() {
    unsafe {
        env::set_var("TEST_VAR", "/test/path");
    }

    let result = match crate::config::expand_path("$TEST_VAR/screenshots") {
        Ok(v) => v,
        Err(err) => panic!("Failed to expand env path: {}", err),
    };
    assert_eq!(result, PathBuf::from("/test/path/screenshots"));

    unsafe {
        env::remove_var("TEST_VAR");
    }
}

#[test]
fn test_expand_path_xdg_pictures() {
    let result = match crate::config::expand_path("$XDG_PICTURES_DIR/screenshots") {
        Ok(v) => v,
        Err(err) => panic!("Failed to expand XDG pictures path: {}", err),
    };
    if let Some(pictures_dir) = dirs::picture_dir() {
        assert_eq!(result, pictures_dir.join("screenshots"));
    } else {
        let home = match dirs::home_dir() {
            Some(v) => v,
            None => panic!("Failed to resolve home directory"),
        };
        assert_eq!(result, home.join("Pictures/screenshots"));
    }
}

#[test]
fn test_expand_path_empty() {
    let result = match crate::config::expand_path("") {
        Ok(v) => v,
        Err(err) => panic!("Failed to expand empty path: {}", err),
    };
    assert_eq!(result, PathBuf::from("."));
}

#[test]
fn test_expand_path_no_expansion() {
    let result = match crate::config::expand_path("/absolute/path") {
        Ok(v) => v,
        Err(err) => panic!("Failed to expand absolute path: {}", err),
    };
    assert_eq!(result, PathBuf::from("/absolute/path"));

    let result = match crate::config::expand_path("relative/path") {
        Ok(v) => v,
        Err(err) => panic!("Failed to expand relative path: {}", err),
    };
    assert_eq!(result, PathBuf::from("relative/path"));
}

#[test]
fn test_expand_path_undefined_var() {
    let result = match crate::config::expand_path("$UNDEFINED_VAR_12345/test") {
        Ok(v) => v,
        Err(err) => panic!("Failed to expand undefined var path: {}", err),
    };
    assert_eq!(result, PathBuf::from("$UNDEFINED_VAR_12345/test"));
}

#[test]
fn test_get_screenshots_dir_priority_cli() {
    let config = crate::config::Config::default();
    let cli_path = Some(PathBuf::from("/cli/path"));

    unsafe {
        env::set_var("HYPRSHOT_DIR", "/env/path");
    }

    let result = match crate::config::get_screenshots_dir(cli_path, &config, false) {
        Ok(v) => v,
        Err(err) => panic!("Failed to resolve screenshots dir (cli): {}", err),
    };
    assert_eq!(result, PathBuf::from("/cli/path"));

    unsafe {
        env::remove_var("HYPRSHOT_DIR");
    }
}

#[test]
fn test_get_screenshots_dir_priority_env() {
    let config = crate::config::Config::default();

    unsafe {
        env::set_var("HYPRSHOT_DIR", "/env/path");
    }

    let result = match crate::config::get_screenshots_dir(None, &config, false) {
        Ok(v) => v,
        Err(err) => panic!("Failed to resolve screenshots dir (env): {}", err),
    };
    assert_eq!(result, PathBuf::from("/env/path"));

    unsafe {
        env::remove_var("HYPRSHOT_DIR");
    }
}

#[test]
fn test_get_screenshots_dir_priority_config() {
    let mut config = crate::config::Config::default();
    config.paths.screenshots_dir = "/config/path".to_string();

    let result = match crate::config::get_screenshots_dir(None, &config, false) {
        Ok(v) => v,
        Err(err) => panic!("Failed to resolve screenshots dir (config): {}", err),
    };
    assert_eq!(result, PathBuf::from("/config/path"));
}

#[test]
fn test_get_screenshots_dir_with_tilde() {
    let mut config = crate::config::Config::default();
    config.paths.screenshots_dir = "~/Screenshots".to_string();

    let result = match crate::config::get_screenshots_dir(None, &config, false) {
        Ok(v) => v,
        Err(err) => panic!("Failed to resolve screenshots dir (tilde): {}", err),
    };
    let home = match dirs::home_dir() {
        Some(v) => v,
        None => panic!("Failed to resolve home directory"),
    };
    assert_eq!(result, home.join("Screenshots"));
}

#[test]
fn test_generate_hyprland_binds() {
    let config = crate::config::Config::default();
    let binds = config.generate_hyprland_binds();

    assert!(binds.contains("# hyprshot-rs keybindings"));
    assert!(binds.contains("# Generated by: hyprshot-rs --generate-hyprland-config"));

    assert!(binds.contains("bind = SUPER, Print, exec, hyprshot-rs -m window"));
    assert!(binds.contains("bind = SUPER SHIFT, Print, exec, hyprshot-rs -m region"));
    assert!(binds.contains("bind = SUPER CTRL, Print, exec, hyprshot-rs -m output"));
    assert!(binds.contains("bind = , Print, exec, hyprshot-rs -m active -m output"));

    assert!(!binds.contains("--clipboard-only"));
}

#[test]
fn test_generate_hyprland_binds_with_clipboard() {
    let config = crate::config::Config::default();
    let binds = config.generate_hyprland_binds_with_clipboard();

    assert!(binds.contains("bind = SUPER, Print, exec, hyprshot-rs -m window"));
    assert!(binds.contains("bind = SUPER SHIFT, Print, exec, hyprshot-rs -m region"));

    assert!(binds.contains("# Screenshot to clipboard (no file saved)"));
    assert!(binds.contains(
        "bind = SUPER ALT, Print, exec, hyprshot-rs -m window --clipboard-only"
    ));
    assert!(binds.contains(
        "bind = SUPER SHIFT ALT, Print, exec, hyprshot-rs -m region --clipboard-only"
    ));
    assert!(binds.contains(
        "bind = SUPER CTRL ALT, Print, exec, hyprshot-rs -m output --clipboard-only"
    ));
}
