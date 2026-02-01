use anyhow::{Context, Result};

use crate::config;

/// Generate Hyprland keybindings
pub fn handle_generate_hyprland_config(with_clipboard: bool) -> Result<()> {
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
    println!(
        "\nOr use: hyprshot-rs --install-binds{}",
        if with_clipboard {
            " --with-clipboard"
        } else {
            ""
        }
    );

    Ok(())
}

/// Install Hyprland keybindings to hyprland.conf
pub fn handle_install_binds(with_clipboard: bool) -> Result<()> {
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
    println!(
        "Backup created: {}",
        installed_path.with_extension("conf.backup").display()
    );

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
pub fn handle_setup_hotkeys() -> Result<()> {
    use dialoguer::{Confirm, Input, theme::ColorfulTheme};

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
        println!(
            "\nConfiguration saved to: {}",
            config::Config::config_path()?.display()
        );

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
