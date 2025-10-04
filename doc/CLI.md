# CLI Reference - hyprshot-rs

Complete command-line interface documentation for hyprshot-rs.

## Table of Contents

- [Basic Usage](#basic-usage)
- [Screenshot Modes](#screenshot-modes)
- [Configuration Management](#configuration-management)
- [Additional Options](#additional-options)
- [Examples](#examples)
- [Configuration File](#configuration-file)

---

## Basic Usage

```bash
hyprshot-rs [OPTIONS] -m <MODE> [-- COMMAND]
```

### Quick Start

```bash
# Capture a region
hyprshot-rs -m region

# Capture active window
hyprshot-rs -m window -m active

# Capture specific monitor
hyprshot-rs -m output
```

---

## Screenshot Modes

Specify one or more modes with `-m` / `--mode`:

### Available Modes

| Mode | Description | Example |
|------|-------------|---------|
| `region` | Select a region with your mouse | `hyprshot-rs -m region` |
| `window` | Select a window | `hyprshot-rs -m window` |
| `output` | Select a monitor | `hyprshot-rs -m output` |
| `active` | Capture active window/monitor | `hyprshot-rs -m window -m active` |
| `OUTPUT_NAME` | Capture specific monitor by name | `hyprshot-rs -m output -m DP-1` |

### Mode Combinations

```bash
# Active window (no selection needed)
hyprshot-rs -m window -m active

# Active monitor (no selection needed)
hyprshot-rs -m output -m active
hyprshot-rs -m active -m output  # Same as above

# Specific monitor by name
hyprshot-rs -m output -m DP-1
```

To get monitor names:
```bash
hyprctl monitors
```

---

## Configuration Management

### Initialize Configuration

Create a default configuration file at `~/.config/hyprshot-rs/config.toml`:

```bash
hyprshot-rs --init-config
```

Output:
```
✓ Config file created at: /home/user/.config/hyprshot-rs/config.toml

Default configuration:
  Screenshots directory: ~/Pictures

You can edit this file manually or use:
  hyprshot-rs --set KEY VALUE

Example:
  hyprshot-rs --set paths.screenshots_dir ~/Documents/Screenshots
```

### View Configuration

Show current configuration:

```bash
hyprshot-rs --show-config
```

Output:
```
Configuration file: /home/user/.config/hyprshot-rs/config.toml

[paths]
screenshots_dir = "~/Pictures"

[hotkeys]
window = "SUPER, Print"
region = "SUPER SHIFT, Print"
...
```

### Get Config Path

Print the path to the config file:

```bash
hyprshot-rs --config-path
```

Output:
```
/home/user/.config/hyprshot-rs/config.toml
```

### Set Configuration Values

Change configuration values from command line:

```bash
hyprshot-rs --set KEY VALUE
```

#### Available Configuration Keys

**Paths:**
```bash
hyprshot-rs --set paths.screenshots_dir ~/Documents/Screenshots
hyprshot-rs --set paths.screenshots_dir '$XDG_PICTURES_DIR/hyprshot'
hyprshot-rs --set paths.screenshots_dir /mnt/storage/screenshots
```

**Hotkeys (for Hyprland configuration):**
```bash
hyprshot-rs --set hotkeys.window "SUPER, Print"
hyprshot-rs --set hotkeys.region "SUPER SHIFT, Print"
hyprshot-rs --set hotkeys.output "SUPER CTRL, Print"
hyprshot-rs --set hotkeys.active_output ", Print"
```

**Capture Settings:**
```bash
# Default format (png, jpeg, ppm)
hyprshot-rs --set capture.default_format png

# Auto-copy to clipboard
hyprshot-rs --set capture.clipboard_on_capture true

# Notifications
hyprshot-rs --set capture.notification true
hyprshot-rs --set capture.notification_timeout 3000
```

**Advanced Settings:**
```bash
# Freeze screen on region selection
hyprshot-rs --set advanced.freeze_on_region true

# Delay before capture (milliseconds)
hyprshot-rs --set advanced.delay_ms 1000
```

---

## Additional Options

### Output Options

| Option | Short | Description | Example |
|--------|-------|-------------|---------|
| `--output-folder` | `-o` | Directory to save screenshot | `-o ~/Screenshots` |
| `--filename` | `-f` | Custom filename | `-f my_screenshot.png` |
| `--raw` | `-r` | Output raw PNG to stdout | `-r > output.png` |
| `--clipboard-only` | | Copy to clipboard without saving | `--clipboard-only` |

### Capture Options

| Option | Short | Description | Example |
|--------|-------|-------------|---------|
| `--delay` | `-D` | Delay before capture (seconds) | `-D 3` |
| `--freeze` | | Freeze screen during selection | `--freeze` |

### Notification Options

| Option | Short | Description | Example |
|--------|-------|-------------|---------|
| `--silent` | `-s` | Don't show notifications | `-s` |
| `--notif-timeout` | `-t` | Notification timeout (ms) | `-t 5000` |

### Other Options

| Option | Short | Description |
|--------|-------|-------------|
| `--debug` | `-d` | Print debug information |
| `--help` | `-h` | Show help message |

### Post-Capture Command

Run a command after capturing:

```bash
hyprshot-rs -m region -- <command>
```

Examples:
```bash
# Open with image viewer
hyprshot-rs -m region -- mirage

# Open with GIMP
hyprshot-rs -m window -- gimp

# Upload to server (custom script)
hyprshot-rs -m output -- ~/bin/upload-screenshot.sh
```

---

## Examples

### Basic Screenshots

```bash
# Capture region and save to ~/Pictures
hyprshot-rs -m region

# Capture active window
hyprshot-rs -m window -m active

# Capture specific monitor
hyprshot-rs -m output -m DP-1
```

### Custom Output Location

```bash
# Save to specific directory
hyprshot-rs -m region -o ~/Documents/Screenshots

# Save with custom filename
hyprshot-rs -m window -f my_window.png

# Save to current directory
hyprshot-rs -m region -o . -f screenshot.png
```

### Clipboard Only

```bash
# Copy region to clipboard (no file saved)
hyprshot-rs -m region --clipboard-only

# Copy active window to clipboard
hyprshot-rs -m window -m active --clipboard-only
```

### Raw Output (Piping)

```bash
# Redirect to file
hyprshot-rs -m region -r > screenshot.png

# Pipe to clipboard tool
hyprshot-rs -m window -r | wl-copy

# Pipe to image processor
hyprshot-rs -m output -r | convert - -resize 50% thumbnail.png
```

### Delayed Capture

```bash
# Wait 3 seconds before capturing
hyprshot-rs -m window -D 3

# Wait 5 seconds with frozen screen
hyprshot-rs -m region -D 5 --freeze
```

### Silent Mode

```bash
# No notifications
hyprshot-rs -m region -s

# Custom notification timeout (2 seconds)
hyprshot-rs -m window -t 2000
```

### Debug Mode

```bash
# Show detailed information
hyprshot-rs -m region -d
```

Output:
```
Using screenshot directory from config: /home/user/Pictures
Saving in: /home/user/Pictures/2024-10-04-201530_hyprshot.png
```

### Combined Examples

```bash
# Screenshot region, wait 2 seconds, no notification, open in viewer
hyprshot-rs -m region -D 2 -s -- eog

# Screenshot active window, save to Desktop, custom filename
hyprshot-rs -m window -m active -o ~/Desktop -f active_window.png

# Screenshot monitor DP-1, copy to clipboard only, with debug
hyprshot-rs -m output -m DP-1 --clipboard-only -d

# Screenshot region with frozen screen, custom location, open in GIMP
hyprshot-rs -m region --freeze -o ~/Work/Screenshots -- gimp
```

---

## Configuration File

### Location

```
~/.config/hyprshot-rs/config.toml
```

### Default Configuration

```toml
[paths]
# Path for saving screenshots
# Supports: ~, $HOME, $XDG_PICTURES_DIR, or absolute paths
screenshots_dir = "~/Pictures"

[hotkeys]
# Hotkey bindings for Hyprland (informational)
window = "SUPER, Print"
region = "SUPER SHIFT, Print"
output = "SUPER CTRL, Print"
active_output = ", Print"

[capture]
# Default image format
default_format = "png"  # png, jpeg, ppm

# Automatically copy to clipboard
clipboard_on_capture = false

# Show notifications
notification = true
notification_timeout = 3000  # milliseconds

[advanced]
# Freeze screen when selecting region
freeze_on_region = true

# Delay before capture
delay_ms = 0  # milliseconds
```

### Path Variables

The `screenshots_dir` setting supports variable expansion:

```toml
# Home directory expansion
screenshots_dir = "~/Pictures"
screenshots_dir = "~/Documents/Screenshots"

# Environment variables
screenshots_dir = "$HOME/Pictures"
screenshots_dir = "$XDG_PICTURES_DIR/hyprshot"

# Absolute paths
screenshots_dir = "/mnt/storage/screenshots"
```

### Priority Order

Screenshot save directory is determined by (highest to lowest priority):

1. **CLI argument**: `-o/--output-folder`
2. **Environment variable**: `$HYPRSHOT_DIR`
3. **Config file**: `paths.screenshots_dir`
4. **Default**: `~/Pictures`

Example:
```bash
# Override config with CLI argument
hyprshot-rs -m region -o /tmp

# Override config with environment variable
export HYPRSHOT_DIR=~/Desktop
hyprshot-rs -m region
```

---

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `HYPRSHOT_DIR` | Override screenshot directory | `export HYPRSHOT_DIR=~/Desktop` |

---

## Hyprland Integration

Add these bindings to your `~/.config/hypr/hyprland.conf`:

```conf
# Screenshot keybindings
bind = SUPER, Print, exec, hyprshot-rs -m window
bind = SUPER SHIFT, Print, exec, hyprshot-rs -m region
bind = SUPER CTRL, Print, exec, hyprshot-rs -m output
bind = , Print, exec, hyprshot-rs -m active -m output

# Screenshot to clipboard
bind = SUPER ALT, Print, exec, hyprshot-rs -m window --clipboard-only
bind = SUPER ALT SHIFT, Print, exec, hyprshot-rs -m region --clipboard-only
```

You can customize these hotkeys in the config file and regenerate them:

```bash
# Edit hotkeys in config
hyprshot-rs --set hotkeys.window "ALT, Print"

# View updated config
hyprshot-rs --show-config
```

---

## Tips & Tricks

### Quick Clipboard Screenshot

Create an alias in your shell config (`~/.bashrc` or `~/.zshrc`):

```bash
alias ss='hyprshot-rs -m region --clipboard-only'
alias ssw='hyprshot-rs -m window -m active --clipboard-only'
```

Usage:
```bash
ss   # Quick region screenshot to clipboard
ssw  # Quick active window to clipboard
```

### Automated Screenshots

```bash
# Screenshot all monitors
for monitor in $(hyprctl monitors -j | jq -r '.[].name'); do
    hyprshot-rs -m output -m $monitor -f "monitor_${monitor}.png"
done

# Scheduled screenshots
watch -n 60 'hyprshot-rs -m output -m active -f "auto_$(date +%H%M%S).png"'
```

### Custom Save Locations Per Use Case

```bash
# Work screenshots
export HYPRSHOT_DIR=~/Work/Screenshots
hyprshot-rs -m region

# Personal screenshots (use config default)
unset HYPRSHOT_DIR
hyprshot-rs -m region

# Temporary screenshots
hyprshot-rs -m window -o /tmp
```

---

## Troubleshooting

### No outputs found

Check your Wayland compositor is running:
```bash
echo $WAYLAND_DISPLAY
hyprctl monitors
```

### Permission denied

Ensure the screenshots directory is writable:
```bash
hyprshot-rs --config-path
cat $(hyprshot-rs --config-path)
ls -la ~/Pictures
```

### Screenshots not saving

Check debug output:
```bash
hyprshot-rs -m region -d
```

---

## See Also

- [Configuration Documentation](CONFIGURATION.md) - Full configuration reference
- [README.md](../README.md) - Project overview and installation
- [TODO.md](../TODO.md) - Development roadmap