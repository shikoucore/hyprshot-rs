use anyhow::{Context, Result};

use crate::config;

pub fn handle_init_config() -> Result<()> {
    let config_path = config::Config::config_path()?;

    if config_path.exists() {
        println!("Config file already exists at: {}", config_path.display());
        println!("Use --show-config to view current configuration");
        return Ok(());
    }

    let config = config::Config::default();
    config.save().context("Failed to save config file")?;

    println!("Config file created at: {}", config_path.display());
    println!("\nDefault configuration:");
    println!("Screenshots directory: {}", config.paths.screenshots_dir);
    println!("\nYou can edit this file manually or use:");
    println!("hyprshot-rs --set KEY VALUE");
    println!("\nExample:");
    println!("hyprshot-rs --set paths.screenshots_dir ~/Documents/Screenshots");

    Ok(())
}

pub fn handle_show_config() -> Result<()> {
    let config = config::Config::load().context("Failed to load config")?;
    let config_path = config::Config::config_path()?;

    println!("Configuration file: {}", config_path.display());
    println!(
        "\n{}",
        toml::to_string_pretty(&config).context("Failed to serialize config")?
    );

    Ok(())
}

pub fn handle_config_path() -> Result<()> {
    let config_path = config::Config::config_path()?;
    println!("{}", config_path.display());
    Ok(())
}

pub fn handle_set_config(args: &[String]) -> Result<()> {
    if args.len() != 2 {
        return Err(anyhow::anyhow!(
            "--set requires exactly 2 arguments: KEY VALUE"
        ));
    }

    let key = &args[0];
    let value = &args[1];

    let mut config = if config::Config::exists() {
        config::Config::load().context("Failed to load config")?
    } else {
        println!("Config file doesn't exist, creating new one...");
        config::Config::default()
    };

    set_config_value(&mut config, key, value)?;

    config.save().context("Failed to save config")?;

    let config_path = config::Config::config_path()?;
    println!("Configuration updated: {} = {}", key, value);
    println!("Config file: {}", config_path.display());

    Ok(())
}

fn set_config_value(config: &mut config::Config, key: &str, value: &str) -> Result<()> {
    let parts: Vec<&str> = key.split('.').collect();

    if parts.len() != 2 {
        return Err(anyhow::anyhow!(
            "Invalid key format. Expected 'section.field', got '{}'",
            key
        ));
    }

    let section = parts[0];
    let field = parts[1];

    match (section, field) {
        // [paths] section
        ("paths", "screenshots_dir") => {
            config.paths.screenshots_dir = value.to_string();
        }

        // [hotkeys] section
        ("hotkeys", "window") => {
            config.hotkeys.window = value.to_string();
        }
        ("hotkeys", "region") => {
            config.hotkeys.region = value.to_string();
        }
        ("hotkeys", "output") => {
            config.hotkeys.output = value.to_string();
        }
        ("hotkeys", "active_output") => {
            config.hotkeys.active_output = value.to_string();
        }

        // [capture] section
        ("capture", "notification") => {
            config.capture.notification =
                value.parse().context("Value must be 'true' or 'false'")?;
        }
        ("capture", "notification_timeout") => {
            config.capture.notification_timeout = value
                .parse()
                .context("Value must be a number (milliseconds)")?;
        }

        // [advanced] section
        ("advanced", "freeze_on_region") => {
            config.advanced.freeze_on_region =
                value.parse().context("Value must be 'true' or 'false'")?;
        }
        ("advanced", "delay_ms") => {
            config.advanced.delay_ms = value
                .parse()
                .context("Value must be a number (milliseconds)")?;
        }

        _ => {
            return Err(anyhow::anyhow!(
                "Unknown config key: {}.{}\n\nAvailable keys:\n\
                 Paths:\n\
                   - paths.screenshots_dir\n\
                 Hotkeys:\n\
                   - hotkeys.window\n\
                   - hotkeys.region\n\
                   - hotkeys.output\n\
                   - hotkeys.active_output\n\
                 Capture:\n\
                   - capture.notification (true, false)\n\
                   - capture.notification_timeout (milliseconds)\n\
                 Advanced:\n\
                   - advanced.freeze_on_region (true, false)\n\
                   - advanced.delay_ms (milliseconds)",
                section,
                field
            ));
        }
    }

    Ok(())
}
