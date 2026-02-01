use anyhow::{Context, Result};
use chrono::Local;
use notify_rust::Notification;
use std::thread::sleep;
use std::time::Duration;

use crate::capture;
use crate::cli::{Args, Mode, default_filename, resolve_delay, resolve_notif_timeout};
use crate::config;
use crate::config_cmds::{
    handle_config_path, handle_init_config, handle_set_config, handle_show_config,
};
use crate::freeze;
use crate::hyprland_cmds::{
    handle_generate_hyprland_config, handle_install_binds, handle_setup_hotkeys,
};
use crate::save;
use crate::utils;

pub fn run(mut args: Args) -> Result<()> {
    // Handle config management commands first
    if args.init_config {
        return handle_init_config();
    }

    if args.show_config {
        return handle_show_config();
    }

    if args.config_path {
        return handle_config_path();
    }

    if let Some(ref set_args) = args.set {
        return handle_set_config(set_args);
    }

    // Handle Hyprland integration commands
    if args.generate_hyprland_config {
        return handle_generate_hyprland_config(args.with_clipboard);
    }

    if args.install_binds {
        return handle_install_binds(args.with_clipboard);
    }

    if args.setup_hotkeys {
        return handle_setup_hotkeys();
    }

    if args.mode.is_empty() {
        print_help();
        return Ok(());
    }

    let debug = args.debug;
    let clipboard_only = args.clipboard_only;
    let raw = args.raw;
    let command = if args.command.is_empty() {
        None
    } else {
        Some(std::mem::take(&mut args.command))
    };

    let mut option: Option<Mode> = None;
    let mut current = false;
    let mut selected_monitor: Option<String> = None;

    let modes = std::mem::take(&mut args.mode);
    for mode in modes {
        match mode {
            Mode::Output | Mode::Window | Mode::Region => {
                option = Some(mode);
            }
            Mode::Active => {
                current = true;
            }
            Mode::OutputName(name) => {
                selected_monitor = Some(name);
            }
        }
    }

    let option = option.context("A mode is required (output, region, window)")?;

    let config = if args.no_config {
        if debug {
            eprintln!("Config loading disabled (--no-config flag)");
        }
        config::Config::default()
    } else {
        config::Config::load().unwrap_or_else(|e| {
            if debug {
                eprintln!("Failed to load config, using defaults: {}", e);
            }
            config::Config::default()
        })
    };

    // Apply settings with priority: CLI > config > default
    let silent = if args.silent {
        true
    } else {
        !config.capture.notification
    };

    let notif_timeout = resolve_notif_timeout(&args, &config);

    let freeze = if args.freeze {
        true
    } else {
        config.advanced.freeze_on_region
    };

    let delay = resolve_delay(&args, &config);

    let save_dir = config::get_screenshots_dir(args.output_folder.clone(), &config, debug)?;

    let save_dir = if !clipboard_only && !raw {
        config::ensure_directory(&save_dir.to_string_lossy())?
    } else {
        save_dir
    };

    let filename = args
        .filename
        .unwrap_or_else(|| default_filename(Local::now()));
    let save_fullpath = save_dir.join(&filename);

    if debug && !clipboard_only {
        eprintln!("Saving in: {}", save_fullpath.display());
    }

    let freeze_guard: Option<freeze::FreezeGuard> = if freeze {
        Some(freeze::start_freeze(selected_monitor.as_deref(), debug)?)
    } else {
        None
    };

    if delay > Duration::from_secs(0) {
        sleep(delay);
    }

    let mut hyprctl_cache = capture::HyprctlCache::new();

    let geometry = match option {
        Mode::Output => {
            if current {
                capture::grab_active_output(debug, &mut hyprctl_cache)?
            } else if let Some(monitor) = selected_monitor {
                capture::grab_selected_output(&monitor, debug)?
            } else {
                capture::grab_output(debug)?
            }
        }
        Mode::Region => match capture::grab_region(debug) {
            Ok(geo) => geo,
            Err(err) => {
                if !silent && err.to_string().contains("slurp failed to select region") {
                    let _ = Notification::new()
                        .summary("Region mode")
                        .body("Drag to select an area (not a window/output).")
                        .appname("Hyprshot-rs")
                        .timeout(notif_timeout as i32)
                        .show();
                }
                return Err(err);
            }
        },
        Mode::Window => {
            let geo = if current {
                capture::grab_active_window(debug)?
            } else {
                capture::grab_window(debug, &mut hyprctl_cache)?
            };
            utils::trim(&geo, debug)?
        }
        _ => unreachable!(),
    };

    if let Some(guard) = freeze_guard {
        guard.stop()?;
    }

    save::save_geometry(
        &geometry,
        &save_fullpath,
        clipboard_only,
        raw,
        command,
        silent,
        notif_timeout,
        debug,
    )?;

    Ok(())
}

fn print_help() {
    println!(
        r#"
Usage: hyprshot-rs [options ..] [-m [mode] ..] -- [command]

Hyprshot-rs is an utility to easily take screenshot in Hyprland using your mouse.

It allows taking screenshots of windows, regions and monitors which are saved to a folder of your choosing and copied to your clipboard.

Examples:
  capture a window                      `hyprshot-rs -m window`
  capture active window to clipboard    `hyprshot-rs -m window -m active --clipboard-only`
  capture selected monitor              `hyprshot-rs -m output -m DP-1`

Options:
  -h, --help                show help message
  -m, --mode                one of: output, window, region, active, OUTPUT_NAME
  -o, --output-folder       directory in which to save screenshot
  -f, --filename            the file name of the resulting screenshot
  -D, --delay               how long to delay taking the screenshot after selection (seconds)
  --freeze                  freeze the screen on initialization
  -d, --debug               print debug information
  -s, --silent              don't send notification when screenshot is saved
  -r, --raw                 output raw image data to stdout
  -n, --notif-timeout       notification timeout in milliseconds (default 5000)
  --clipboard-only          copy screenshot to clipboard and don't save image in disk
  --no-config               don't load config file (use defaults and CLI args only)
  -- [command]              open screenshot with a command of your choosing. e.g. hyprshot-rs -m window -- mirage

Config Management:
  --init-config             initialize default config file (~/.config/hyprshot-rs/config.toml)
  --show-config             show current configuration
  --config-path             show path to config file
  --set KEY VALUE           set config value (e.g., --set paths.screenshots_dir ~/Screenshots)

Hyprland Integration:
  --generate-hyprland-config    generate keybindings for Hyprland
  --install-binds               install keybindings to hyprland.conf (creates backup)
  --with-clipboard              include clipboard-only variants (use with above commands)
  --setup-hotkeys               interactive wizard to configure hotkeys

Modes:
  output        take screenshot of an entire monitor
  window        take screenshot of an open window
  region        take screenshot of selected region
  active        take screenshot of active window|output
                (you must use --mode again with the intended selection)
  OUTPUT_NAME   take screenshot of output with OUTPUT_NAME
                (you must use --mode again with the intended selection)
                (you can get this from `hyprctl monitors`)
"#
    );
}
