[![Crates.io Version](https://img.shields.io/crates/v/hyprshot-rs.svg)](https://crates.io/crates/hyprshot-rs) [![Crates.io Downloads](https://img.shields.io/crates/d/hyprshot-rs.svg)](https://crates.io/crates/hyprshot-rs) [![AUR version](https://img.shields.io/aur/version/hyprshot-rs)](https://aur.archlinux.org/packages/hyprshot-rs) [![Crates.io License](https://img.shields.io/crates/l/hyprshot-rs.svg)](https://crates.io/crates/hyprshot-rs) [![Rust](https://github.com/vremyavnikuda/hyprshot-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/vremyavnikuda/hyprshot-rs/actions/workflows/rust.yml)
___
>if you like this project, then the best way to express gratitude is to give it a star ⭐, it doesn't cost you anything, but I understand that I'm moving the project in the right direction.

# Hyprshot-rs
___
# release version 0.1.4

A utility to easily take screenshots in Hyprland using your mouse.

## Features
- **Screenshot Capture**
    - Capture the entire monitor (output)
    - Capture the active monitor (output + active)
    - Capturing the selected region
    - Capturing the selected window
    - Capture of the active window
- **Save & Clipboard**
    - Save screenshots to a specified folder or copy to clipboard
    - Screenshots saved in PNG format
- **Configuration System**
    - TOML-based configuration (`~/.config/hyprshot-rs/config.toml`)
    - Persistent settings for paths, hotkeys, notifications, and more
    - CLI commands for config management (`--init-config`, `--show-config`, `--set`)
- **Hyprland Integration**
    - Automatic keybinding generation (`--generate-hyprland-config`)
    - One-command installation to hyprland.conf (`--install-binds`)
    - Interactive hotkeys setup wizard (`--setup-hotkeys`)
- **Documentation**
    - Complete [CLI reference](doc/CLI.md)
    - [Configuration guide](doc/CONFIGURATION.md)
    - [Hotkeys setup guide](doc/HOTKEYS.md)

## Installation

### Via Cargo:
```bash
cargo install hyprshot-rs
```

### Via AUR (Arch Linux):
```bash
yay -S hyprshot-rs
```

### Runtime Dependencies

**Required:**
- `wl-clipboard` - for clipboard operations
- `hyprland` - the compositor

**Optional:**
- `hyprpicker` - for screen freeze functionality (`--freeze` flag)

On Arch Linux:
```bash
sudo pacman -S wl-clipboard hyprland hyprpicker
```

> **Note:** Starting from v0.1.4, `slurp` is embedded into the binary and no longer needs to be installed separately! If you have `slurp` installed system-wide, hyprshot-rs will use it; otherwise, it will use the embedded version automatically.
___
## Usage
Make it available regardless of the shell
```bash
sudo ln -s ~/.local/share/cargo/bin/hyprshot-rs /usr/local/bin/
```

```bash
hyprshot-rs [options ..] [-m [mode] ..] -- [command]
```
```
possible values: output, window, region, active, OUTPUT_NAME
```

Note: `active` is a modifier and must be combined with `output` or `window`.

Possible values:
- Capture a window:
```bash
hyprshot-rs -m window
```
- To take a screenshot of a specific area of the screen, use:
```bash
hyprshot-rs -m region
```
- If you have 2 or more monitors and want to take a screenshot of the workspace on a specific monitor: 
```bash
hyprshot-rs -m output
```
- Quick capture (active monitor under the cursor):
```bash
hyprshot-rs -m output -m active
```
- Capture the active window:
```bash
hyprshot-rs -m window -m active
```
- Capture a specific monitor by name (example: `DP-1`):
```bash
hyprshot-rs -m output -m DP-1
```
- Take a screenshot of a selected area and save it in the current directory:
~/repository
```bash
hyprshot-rs -m region -r > output.png
```
redirects the output to output.png in your current working directory. So if you're currently in ~/repository when running this command, that's where the screenshot will be saved, not in the default ~/Pictures directory.


Run `hyprshot-rs --help` or `hyprshot-rs -h` for more options.

## Configuration

Initialize configuration:
```bash
hyprshot-rs --init-config
```

Configure settings:
```bash
hyprshot-rs --set paths.screenshots_dir ~/user_name/Screenshots
hyprshot-rs --set capture.notification_timeout 500
```

View current configuration:
```bash
hyprshot-rs --show-config
```

## Hyprland Integration

**Quick setup with interactive wizard:**
```bash
hyprshot-rs --setup-hotkeys
```

**Or manually generate and install keybindings:**
```bash
# Generate keybindings
hyprshot-rs --generate-hyprland-config --with-clipboard

# Install to hyprland.conf (creates backup)
hyprshot-rs --install-binds --with-clipboard
```

**Manual configuration** - add to hyprland.conf:
```cfg
bind = SUPER, Print, exec, hyprshot-rs -m window
bind = SUPER SHIFT, Print, exec, hyprshot-rs -m region
bind = SUPER CTRL, Print, exec, hyprshot-rs -m output
bind = , Print, exec, hyprshot-rs -m output -m active
```

See [HOTKEYS.md](doc/HOTKEYS.md) for more examples.
Based on the implementation: [Hypershot](https://github.com/Gustash/Hyprshot)
## License
[GPL-3.0](LICENSE.md)
