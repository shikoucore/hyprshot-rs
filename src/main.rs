use anyhow::{ Context, Result };
use chrono::Local;
use clap::{ Parser, ValueEnum };
use std::path::PathBuf;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use toml;

mod capture;
mod config;
mod save;
mod utils;

#[derive(Parser)]
#[command(name = "hyprshot-rs", about = "Utility to easily take screenshots in Hyprland")]
struct Args {
    #[arg(short = 'm', long, help = "Mode: output, window, region, active, or OUTPUT_NAME")]
    mode: Vec<Mode>,

    #[arg(short, long, help = "Directory to save screenshot")]
    output_folder: Option<PathBuf>,

    #[arg(short, long, help = "Filename of the screenshot")]
    filename: Option<String>,

    #[arg(short = 'D', long, help = "Delay before taking screenshot (seconds)")]
    delay: Option<u64>,

    #[arg(long, help = "Freeze the screen on initialization")]
    freeze: bool,

    #[arg(short, long, help = "Print debug information")]
    debug: bool,

    #[arg(short, long, help = "Don't send notification")]
    silent: bool,

    #[arg(short, long, help = "Output raw image data to stdout")]
    raw: bool,

    #[arg(short, long, default_value = "5000", help = "Notification timeout (ms)")]
    notif_timeout: u32,

    #[arg(long, help = "Copy to clipboard and don't save to disk")]
    clipboard_only: bool,

    #[arg(last = true, help = "Command to open screenshot (e.g., 'mirage')")]
    command: Vec<String>,

    // Config management commands
    #[arg(long, help = "Initialize default config file")]
    init_config: bool,

    #[arg(long, help = "Show current configuration")]
    show_config: bool,

    #[arg(long, help = "Show path to config file")]
    config_path: bool,

    #[arg(
        long,
        value_names = ["KEY", "VALUE"],
        num_args = 2,
        help = "Set config value (e.g., --set paths.screenshots_dir ~/Screenshots)"
    )]
    set: Option<Vec<String>>,

    // Hyprland integration commands
    #[arg(long, help = "Generate Hyprland keybindings")]
    generate_hyprland_config: bool,

    #[arg(long, help = "Install keybindings to hyprland.conf (creates backup)")]
    install_binds: bool,

    #[arg(long, help = "Include clipboard-only bindings when generating")]
    with_clipboard: bool,

    #[arg(long, help = "Interactive hotkeys setup wizard")]
    setup_hotkeys: bool,

    #[arg(long, help = "Don't load configuration file (use defaults and CLI args only)")]
    no_config: bool,
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

#[derive(Clone, Debug, ValueEnum)]
enum Mode {
    Output,
    Window,
    Region,
    Active,
    #[clap(skip)] OutputName(String),
}

fn main() -> Result<()> {
    let args = Args::parse();

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
    let command = if args.command.is_empty() { None } else { Some(args.command) };

    let mut option: Option<Mode> = None;
    let mut current = false;
    let mut selected_monitor: Option<String> = None;

    for mode in args.mode {
        match mode {
            Mode::Output | Mode::Window | Mode::Region => {
                option = Some(mode);
            }
            Mode::Active => {
                current = true;
            }
            Mode::OutputName(name) => {
                if utils::is_valid_monitor(&name)? {
                    selected_monitor = Some(name);
                }
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
    
    // Notification settings
    let silent = if args.silent {
        true
    } else {
        !config.capture.notification
    };
    
    let notif_timeout = if args.notif_timeout != 5000 {
        args.notif_timeout
    } else {
        config.capture.notification_timeout
    };
    
    let freeze = if args.freeze {
        true
    } else {
        config.advanced.freeze_on_region
    };
    
    let delay = if let Some(d) = args.delay {
        d
    } else if config.advanced.delay_ms > 0 {
        ((config.advanced.delay_ms + 999) / 1000) as u64
    } else {
        0u64
    };

    let save_dir = config::get_screenshots_dir(args.output_folder.clone(), &config, debug)?;

    let save_dir = if !clipboard_only && !raw {
        config::ensure_directory(&save_dir.to_string_lossy())?
    } else {
        save_dir
    };

    let filename = args.filename.unwrap_or_else(|| {
        Local::now().format("%Y-%m-%d-%H%M%S_hyprshot.png").to_string()
    });
    let save_fullpath = save_dir.join(&filename);

    if debug && !clipboard_only {
        eprintln!("Saving in: {}", save_fullpath.display());
    }

    let hyprpicker_pid = if freeze && Command::new("hyprpicker").output().is_ok() {
        let pid = Command::new("hyprpicker")
            .args(["-r", "-z"])
            .spawn()
            .context("Failed to start hyprpicker")?
            .id();
        sleep(Duration::from_millis(200));
        Some(pid)
    } else {
        None
    };

    if delay > 0 {
        sleep(Duration::from_secs(delay));
    }

    let geometry = match option {
        Mode::Output => {
            if current {
                capture::grab_active_output(debug)?
            } else if let Some(monitor) = selected_monitor {
                capture::grab_selected_output(&monitor, debug)?
            } else {
                capture::grab_output(debug)?
            }
        }
        Mode::Region => capture::grab_region(debug)?,
        Mode::Window => {
            let geo = if current {
                capture::grab_active_window(debug)?
            } else {
                capture::grab_window(debug)?
            };
            utils::trim(&geo, debug)?
        }
        _ => unreachable!(),
    };

    save::save_geometry(
        &geometry,
        &save_fullpath,
        clipboard_only,
        raw,
        command,
        silent,
        notif_timeout,
        debug
    )?;

    if let Some(pid) = hyprpicker_pid {
        Command::new("kill").arg(pid.to_string()).status().context("Failed to kill hyprpicker")?;
    }

    Ok(())
}

fn handle_init_config() -> Result<()> {
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

fn handle_show_config() -> Result<()> {
    let config = config::Config::load().context("Failed to load config")?;
    let config_path = config::Config::config_path()?;

    println!("Configuration file: {}", config_path.display());
    println!("\n{}", toml::to_string_pretty(&config).context("Failed to serialize config")?);

    Ok(())
}

fn handle_config_path() -> Result<()> {
    let config_path = config::Config::config_path()?;
    println!("{}", config_path.display());
    Ok(())
}

fn handle_set_config(args: &[String]) -> Result<()> {
    if args.len() != 2 {
        return Err(anyhow::anyhow!("--set requires exactly 2 arguments: KEY VALUE"));
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
        return Err(anyhow::anyhow!("Invalid key format. Expected 'section.field', got '{}'", key));
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
        ("capture", "default_format") => {
            if !["png", "jpeg", "ppm"].contains(&value) {
                return Err(
                    anyhow::anyhow!("Invalid format '{}'. Must be one of: png, jpeg, ppm", value)
                );
            }
            config.capture.default_format = value.to_string();
        }
        ("capture", "clipboard_on_capture") => {
            config.capture.clipboard_on_capture = value
                .parse()
                .context("Value must be 'true' or 'false'")?;
        }
        ("capture", "notification") => {
            config.capture.notification = value.parse().context("Value must be 'true' or 'false'")?;
        }
        ("capture", "notification_timeout") => {
            config.capture.notification_timeout = value
                .parse()
                .context("Value must be a number (milliseconds)")?;
        }

        // [advanced] section
        ("advanced", "freeze_on_region") => {
            config.advanced.freeze_on_region = value
                .parse()
                .context("Value must be 'true' or 'false'")?;
        }
        ("advanced", "delay_ms") => {
            config.advanced.delay_ms = value
                .parse()
                .context("Value must be a number (milliseconds)")?;
        }

        _ => {
            return Err(
                anyhow::anyhow!(
                    "Unknown config key: {}.{}\n\nAvailable keys:\n\
                 Paths:\n\
                   - paths.screenshots_dir\n\
                 Hotkeys:\n\
                   - hotkeys.window\n\
                   - hotkeys.region\n\
                   - hotkeys.output\n\
                   - hotkeys.active_output\n\
                 Capture:\n\
                   - capture.default_format (png, jpeg, ppm)\n\
                   - capture.clipboard_on_capture (true, false)\n\
                   - capture.notification (true, false)\n\
                   - capture.notification_timeout (milliseconds)\n\
                 Advanced:\n\
                   - advanced.freeze_on_region (true, false)\n\
                   - advanced.delay_ms (milliseconds)",
                    section,
                    field
                )
            );
        }
    }

    Ok(())
}

/// Generate Hyprland keybindings
fn handle_generate_hyprland_config(with_clipboard: bool) -> Result<()> {
    let config = config::Config::load()?;

    let binds = if with_clipboard {
        config.generate_hyprland_binds_with_clipboard()
    } else {
        config.generate_hyprland_binds()
    };

    println!("{}", binds);
    println!("\nTo install these bindings:");
    println!("1. Copy the output above");
    println!("2. Paste into ~/.config/hypr/hyprland.conf");
    println!("3. Reload Hyprland config: hyprctl reload");
    println!("\nOr use: hyprshot-rs --install-binds{}", if with_clipboard {
        " --with-clipboard"
    } else {
        ""
    });

    Ok(())
}

/// Install Hyprland keybindings to hyprland.conf
fn handle_install_binds(with_clipboard: bool) -> Result<()> {
    let config = config::Config::load()?;

    let hyprland_conf = config::Config::hyprland_config_path()?;

    if !hyprland_conf.exists() {
        anyhow::bail!(
            "Hyprland config not found at: {}\n\n\
            Please ensure:\n\
            1. Hyprland is installed\n\
            2. Config file exists at ~/.config/hypr/hyprland.conf\n\
            3. You have permission to read/write the file",
            hyprland_conf.display()
        );
    }

    println!("Installing hyprshot-rs keybindings to Hyprland config...\n");

    let installed_path = config
        .install_hyprland_binds(with_clipboard)
        .context("Failed to install keybindings")?;

    println!("Keybindings installed successfully!");
    println!("Config file: {}", installed_path.display());
    println!("Backup created: {}", installed_path.with_extension("conf.backup").display());

    if with_clipboard {
        println!("\nInstalled bindings (with clipboard variants):");
    } else {
        println!("\nInstalled bindings:");
    }

    let binds = if with_clipboard {
        config.generate_hyprland_binds_with_clipboard()
    } else {
        config.generate_hyprland_binds()
    };

    for line in binds.lines().skip(2) {
        if !line.is_empty() {
            println!("  {}", line);
        }
    }

    println!("\nTo apply the changes:");
    println!("hyprctl reload");
    println!("\nOr restart Hyprland.");

    Ok(())
}

/// Interactive hotkeys setup wizard
fn handle_setup_hotkeys() -> Result<()> {
    use dialoguer::{ theme::ColorfulTheme, Input, Confirm };

    println!("This wizard will help you configure hotkeys for hyprshot-rs.");
    println!("Format: \"MODIFIER, KEY\" (e.g., \"SUPER, Print\", \"ALT SHIFT, S\")");
    println!();

    let mut config = config::Config::load().unwrap_or_else(|_| config::Config::default());

    let theme = ColorfulTheme::default();

    println!("Window Screenshot");
    println!("Capture a selected window");
    let window_hotkey: String = Input::with_theme(&theme)
        .with_prompt("Hotkey")
        .default(config.hotkeys.window.clone())
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.contains(',') {
                Ok(())
            } else {
                Err("Hotkey must be in format 'MODIFIER, KEY' (e.g., 'SUPER, Print')")
            }
        })
        .interact_text()?;
    config.hotkeys.window = window_hotkey;
    println!();

    // Configure region screenshot hotkey
    println!("Region Screenshot");
    println!("Capture a selected region");
    let region_hotkey: String = Input::with_theme(&theme)
        .with_prompt("Hotkey")
        .default(config.hotkeys.region.clone())
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.contains(',') {
                Ok(())
            } else {
                Err("Hotkey must be in format 'MODIFIER, KEY'")
            }
        })
        .interact_text()?;
    config.hotkeys.region = region_hotkey;
    println!();

    println!("Output Screenshot");
    println!("Capture entire monitor");
    let output_hotkey: String = Input::with_theme(&theme)
        .with_prompt("Hotkey")
        .default(config.hotkeys.output.clone())
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.contains(',') {
                Ok(())
            } else {
                Err("Hotkey must be in format 'MODIFIER, KEY'")
            }
        })
        .interact_text()?;
    config.hotkeys.output = output_hotkey;
    println!();

    println!("Active Output Screenshot");
    println!("Quick capture of active monitor");
    let active_output_hotkey: String = Input::with_theme(&theme)
        .with_prompt("Hotkey")
        .default(config.hotkeys.active_output.clone())
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.contains(',') {
                Ok(())
            } else {
                Err("Hotkey must be in format 'MODIFIER, KEY' (can be empty modifier: ', Print')")
            }
        })
        .interact_text()?;
    config.hotkeys.active_output = active_output_hotkey;
    println!();

    println!("Configuration Summary:");
    println!("Window Screenshot:{}", config.hotkeys.window);
    println!("Region Screenshot:{}", config.hotkeys.region);
    println!("Output Screenshot:{}", config.hotkeys.output);
    println!("Active Output Screenshot:{}", config.hotkeys.active_output);

    if Confirm::with_theme(&theme)
        .with_prompt("Save this configuration?")
        .default(true)
        .interact()?
    {
        config.save()?;
        println!("\nConfiguration saved to: {}", config::Config::config_path()?.display());

        println!();
        if Confirm::with_theme(&theme)
            .with_prompt("Generate Hyprland keybindings now?")
            .default(true)
            .interact()?
        {
            println!();
            if Confirm::with_theme(&theme)
                .with_prompt("Include clipboard-only variants (with ALT modifier)?")
                .default(true)
                .interact()?
            {
                handle_generate_hyprland_config(true)?;
            } else {
                handle_generate_hyprland_config(false)?;
            }

            println!();
            if Confirm::with_theme(&theme)
                .with_prompt("Install keybindings to hyprland.conf now?")
                .default(false)
                .interact()?
            {
                println!();
                let with_clipboard = Confirm::with_theme(&theme)
                    .with_prompt("Include clipboard variants?")
                    .default(true)
                    .interact()?;

                handle_install_binds(with_clipboard)?;
            }
        }

        println!("• View config:     hyprshot-rs --show-config");
        println!("• Generate binds:  hyprshot-rs --generate-hyprland-config");
        println!("• Install binds:   hyprshot-rs --install-binds");
        println!("• Run setup again: hyprshot-rs --setup-hotkeys");
    } else {
        println!("\nConfiguration not saved.");
    }

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
  -z, --freeze              freeze the screen on initialization
  -d, --debug               print debug information
  -s, --silent              don't send notification when screenshot is saved
  -r, --raw                 output raw image data to stdout
  -t, --notif-timeout       notification timeout in milliseconds (default 5000)
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
