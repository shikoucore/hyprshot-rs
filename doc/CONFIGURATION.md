# Configuration Guide - hyprshot-rs

Complete configuration reference for hyprshot-rs.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Configuration File Location](#configuration-file-location)
- [Configuration Structure](#configuration-structure)
- [Section: Paths](#section-paths)
- [Section: Hotkeys](#section-hotkeys)
- [Section: Capture](#section-capture)
- [Section: Advanced](#section-advanced)
- [Path Expansion](#path-expansion)
- [Priority System](#priority-system)
- [Managing Configuration](#managing-configuration)
- [Configuration Examples](#configuration-examples)
- [Migration Guide](#migration-guide)
- [Troubleshooting](#troubleshooting)

---

## Overview

hyprshot-rs uses a TOML configuration file to store user preferences and settings. The configuration system provides:

- **Persistent settings** - Save your preferred screenshot directory, formats, and behavior
- **Flexible path handling** - Support for `~`, environment variables, and absolute paths
- **Priority system** - CLI arguments override environment variables, which override config file
- **Easy management** - CLI commands for viewing and editing configuration
- **Hotkey management** - Store Hyprland keybindings (for reference/generation)

---

## Quick Start

Initialize configuration with default values:

```bash
hyprshot-rs --init-config
```

This creates `~/.config/hyprshot-rs/config.toml` with sensible defaults.

View your current configuration:

```bash
hyprshot-rs --show-config
```

Change a setting:

```bash
hyprshot-rs --set paths.screenshots_dir ~/Documents/Screenshots
```

---

## Configuration File Location

### Default Location

```
~/.config/hyprshot-rs/config.toml
```

This follows the XDG Base Directory specification.

### Finding Your Config

Get the path to your configuration file:

```bash
hyprshot-rs --config-path
```

### Manual Editing

You can edit the configuration file directly with any text editor:

```bash
# Using your preferred editor
$EDITOR ~/.config/hyprshot-rs/config.toml

# Or specific editors
nano ~/.config/hyprshot-rs/config.toml
vim ~/.config/hyprshot-rs/config.toml
code ~/.config/hyprshot-rs/config.toml
```

---

## Configuration Structure

The configuration file is divided into four logical sections:

```toml
[paths]         # File system paths (screenshots directory)
[hotkeys]       # Hyprland keybinding definitions
[capture]       # Screenshot capture settings
[advanced]      # Advanced behavior options
```

### Default Configuration

```toml
[paths]
screenshots_dir = "~/Pictures"

[hotkeys]
window = "SUPER, Print"
region = "SUPER SHIFT, Print"
output = "SUPER CTRL, Print"
active_output = ", Print"

[capture]
default_format = "png"
clipboard_on_capture = false
notification = true
notification_timeout = 3000

[advanced]
freeze_on_region = true
delay_ms = 0
```

---

## Section: Paths

Controls where screenshots are saved.

### `screenshots_dir`

**Type:** String (path)  
**Default:** `"~/Pictures"`  
**CLI Key:** `paths.screenshots_dir`

The directory where screenshots will be saved by default.

#### Supported Path Formats

```toml
# Home directory expansion
screenshots_dir = "~/Pictures"
screenshots_dir = "~/Documents/Screenshots"
screenshots_dir = "~/Desktop"

# Environment variables
screenshots_dir = "$HOME/Pictures"
screenshots_dir = "$XDG_PICTURES_DIR"
screenshots_dir = "$XDG_PICTURES_DIR/hyprshot"
screenshots_dir = "$CUSTOM_VAR/screenshots"

# Absolute paths
screenshots_dir = "/home/username/Pictures"
screenshots_dir = "/mnt/storage/screenshots"
screenshots_dir = "/tmp/screenshots"
```

#### Path Expansion

All paths are automatically expanded:
- `~` → User's home directory (`/home/username`)
- `$VAR` → Environment variable value
- `$HOME` → User's home directory
- `$XDG_PICTURES_DIR` → XDG Pictures directory (usually `~/Pictures`)

See [Path Expansion](#path-expansion) section for details.

#### Directory Creation

If the specified directory doesn't exist, hyprshot-rs will:
1. Attempt to create it automatically
2. Create parent directories if needed
3. Validate write permissions
4. Show an error if creation fails

#### CLI Examples

```bash
# Set to Documents folder
hyprshot-rs --set paths.screenshots_dir ~/Documents/Screenshots

# Set to XDG Pictures directory
hyprshot-rs --set paths.screenshots_dir '$XDG_PICTURES_DIR'

# Set to absolute path
hyprshot-rs --set paths.screenshots_dir /mnt/storage/screenshots

# Set to current directory
hyprshot-rs --set paths.screenshots_dir .
```

---

## Section: Hotkeys

Defines Hyprland keybinding strings for reference or generation.

**Note:** This section is primarily informational. You must manually add bindings to your `hyprland.conf` file. These values can be used for generating configuration snippets.

### `window`

**Type:** String  
**Default:** `"SUPER, Print"`  
**CLI Key:** `hotkeys.window`

Keybinding for capturing a window screenshot.

```toml
window = "SUPER, Print"          # Default
window = "ALT, Print"            # Alternative
window = "SUPER SHIFT, W"        # Custom
```

### `region`

**Type:** String  
**Default:** `"SUPER SHIFT, Print"`  
**CLI Key:** `hotkeys.region`

Keybinding for capturing a region screenshot.

```toml
region = "SUPER SHIFT, Print"    # Default
region = "ALT SHIFT, Print"      # Alternative
region = "SUPER, S"              # Custom
```

### `output`

**Type:** String  
**Default:** `"SUPER CTRL, Print"`  
**CLI Key:** `hotkeys.output`

Keybinding for capturing a monitor/output screenshot.

```toml
output = "SUPER CTRL, Print"     # Default
output = "ALT CTRL, Print"       # Alternative
output = "SUPER, O"              # Custom
```

### `active_output`

**Type:** String  
**Default:** `", Print"`  
**CLI Key:** `hotkeys.active_output`

Keybinding for capturing the active monitor screenshot.

```toml
active_output = ", Print"        # Default (just Print key)
active_output = "SHIFT, Print"   # With modifier
active_output = "SUPER, A"       # Custom
```

### Using Hotkeys in Hyprland

Add these to your `~/.config/hypr/hyprland.conf`:

```conf
# Read values from your config
bind = SUPER, Print, exec, hyprshot-rs -m window
bind = SUPER SHIFT, Print, exec, hyprshot-rs -m region
bind = SUPER CTRL, Print, exec, hyprshot-rs -m output
bind = , Print, exec, hyprshot-rs -m active -m output

# Clipboard variants
bind = SUPER ALT, Print, exec, hyprshot-rs -m window --clipboard-only
bind = SUPER ALT SHIFT, Print, exec, hyprshot-rs -m region --clipboard-only
```

### CLI Examples

```bash
# Change window hotkey
hyprshot-rs --set hotkeys.window "ALT, Print"

# Change region hotkey
hyprshot-rs --set hotkeys.region "SUPER, S"

# View all hotkeys
hyprshot-rs --show-config | grep -A 5 '\[hotkeys\]'
```

---

## Section: Capture

Controls screenshot capture behavior and format.

### `default_format`

**Type:** String (enum)  
**Default:** `"png"`  
**Valid Values:** `png`, `jpeg`, `ppm` (reserved)  
**CLI Key:** `capture.default_format`

Default image format for saved screenshots.

Note: output is currently always PNG. This setting is reserved for future formats.

```toml
default_format = "png"    # Current output format
default_format = "jpeg"   # Reserved (not used yet)
default_format = "ppm"    # Reserved (not used yet)
```

#### Format Comparison

| Format | Compression | Quality | File Size | Use Case |
|--------|-------------|---------|-----------|----------|
| `png` | Lossless | High | Large | Default, best quality |
| `jpeg` | Lossy | Good | Small | Sharing, space-constrained |
| `ppm` | None | High | Very Large | Raw processing |

**Note:** This setting is currently ignored. PNG is always used.

#### CLI Examples

```bash
hyprshot-rs --set capture.default_format png
hyprshot-rs --set capture.default_format jpeg
```

### `clipboard_on_capture`

**Type:** Boolean  
**Default:** `false`  
**CLI Key:** `capture.clipboard_on_capture`

Automatically copy screenshots to clipboard after capture.

Note: this setting is not wired yet. Use `--clipboard-only` on the CLI.

```toml
clipboard_on_capture = false   # Reserved (currently ignored)
clipboard_on_capture = true    # Reserved (currently ignored)
```

When `true`:
- Reserved (no effect yet)

When `false`:
- Reserved (no effect yet)

#### CLI Examples

```bash
# Enable auto-clipboard
hyprshot-rs --set capture.clipboard_on_capture true

# Disable auto-clipboard
hyprshot-rs --set capture.clipboard_on_capture false
```

### `notification`

**Type:** Boolean  
**Default:** `true`  
**CLI Key:** `capture.notification`

Show desktop notifications when screenshots are captured.

```toml
notification = true    # Show notifications (default)
notification = false   # Silent mode, no notifications
```

When `true`:
- Shows notification with screenshot preview
- Displays file path
- Can be clicked to open screenshot

When `false`:
- No notifications shown
- Same as using `--silent` flag

#### CLI Examples

```bash
# Enable notifications
hyprshot-rs --set capture.notification true

# Disable notifications (silent mode)
hyprshot-rs --set capture.notification false
```

### `notification_timeout`

**Type:** Integer  
**Default:** `3000`  
**Unit:** Milliseconds  
**CLI Key:** `capture.notification_timeout`

Duration for which notifications are displayed.

```toml
notification_timeout = 3000   # 3 seconds (default)
notification_timeout = 5000   # 5 seconds
notification_timeout = 1000   # 1 second
notification_timeout = 10000  # 10 seconds
```

#### CLI Examples

```bash
# Set to 5 seconds
hyprshot-rs --set capture.notification_timeout 5000

# Set to 1 second (quick)
hyprshot-rs --set capture.notification_timeout 1000

# Set to 10 seconds (persistent)
hyprshot-rs --set capture.notification_timeout 10000
```

---

## Section: Advanced

Advanced behavior and timing options.

### `freeze_on_region`

**Type:** Boolean  
**Default:** `true`  
**CLI Key:** `advanced.freeze_on_region`

Freeze the screen when selecting a region for screenshot.

```toml
freeze_on_region = true    # Freeze screen during selection (default)
freeze_on_region = false   # Live screen during selection
```

When `true`:
- Screen is frozen using `hyprpicker --autocopy --format hex --no-fancy`
- Easier to select moving content
- Same as using `--freeze` flag

When `false`:
- Screen remains live during selection
- Shows real-time updates
- Better for capturing animations

**Requirements:** This feature requires `hyprpicker` to be installed.

#### CLI Examples

```bash
# Enable screen freeze
hyprshot-rs --set advanced.freeze_on_region true

# Disable screen freeze
hyprshot-rs --set advanced.freeze_on_region false
```

### `delay_ms`

**Type:** Integer  
**Default:** `0`  
**Unit:** Milliseconds  
**CLI Key:** `advanced.delay_ms`

Delay before capturing the screenshot after selection.

```toml
delay_ms = 0       # No delay (default)
delay_ms = 1000    # 1 second delay
delay_ms = 3000    # 3 seconds delay
delay_ms = 5000    # 5 seconds delay
```

Use cases:
- **0ms (default)**: Immediate capture
- **1-2 seconds**: Time to dismiss menus or hover effects
- **3-5 seconds**: Time to prepare application state
- **5+ seconds**: Time to switch windows or set up scene

**Note:** This is separate from the `--delay` CLI flag, which is in seconds.

#### CLI Examples

```bash
# No delay
hyprshot-rs --set advanced.delay_ms 0

# 1 second delay
hyprshot-rs --set advanced.delay_ms 1000

# 3 seconds delay
hyprshot-rs --set advanced.delay_ms 3000
```

---

## Path Expansion

hyprshot-rs automatically expands paths to their full absolute paths.

### Tilde Expansion

```toml
screenshots_dir = "~/Pictures"
# Expands to: /home/username/Pictures

screenshots_dir = "~/Documents/Work/Screenshots"
# Expands to: /home/username/Documents/Work/Screenshots
```

### Environment Variables

```toml
screenshots_dir = "$HOME/Pictures"
# Expands to: /home/username/Pictures

screenshots_dir = "$XDG_PICTURES_DIR"
# Expands to: /home/username/Pictures (typical)

screenshots_dir = "$XDG_PICTURES_DIR/hyprshot"
# Expands to: /home/username/Pictures/hyprshot

screenshots_dir = "$CUSTOM_VAR/screenshots"
# Expands to: (value of CUSTOM_VAR)/screenshots
```

### Special Variables

| Variable | Typical Value | Description |
|----------|---------------|-------------|
| `$HOME` | `/home/username` | User's home directory |
| `$XDG_PICTURES_DIR` | `~/Pictures` | XDG Pictures directory |
| `$USER` | `username` | Current username |
| Custom variables | Varies | Any environment variable |

### Undefined Variables

If an environment variable is undefined:
- The variable is left as-is in the path
- Example: `$UNDEFINED/screenshots` → `$UNDEFINED/screenshots`
- May cause errors when trying to create/access directory

### Path Validation

hyprshot-rs validates paths:
1. **Expansion**: Resolves `~` and `$VAR`
2. **Existence**: Checks if directory exists
3. **Creation**: Creates directory if missing
4. **Permissions**: Validates write access
5. **Error Handling**: Shows clear error messages

---

## Priority System

Screenshot save location is determined by priority (highest to lowest):

### 1. CLI Argument (Highest Priority)

```bash
hyprshot-rs -m region -o /tmp/screenshots
```

Overrides all other settings.

### 2. Environment Variable

```bash
export HYPRSHOT_DIR=~/Desktop
hyprshot-rs -m region
```

Overrides config file, but not CLI argument.

### 3. Config File

```toml
[paths]
screenshots_dir = "~/Documents/Screenshots"
```

Used when no CLI argument or environment variable is set.

### 4. Default (Lowest Priority)

```
~/Pictures
```

Used when nothing else is configured.

### Priority Examples

```bash
# Scenario 1: Only config file
# Config: screenshots_dir = "~/Documents"
hyprshot-rs -m region
# Result: Saves to ~/Documents

# Scenario 2: Config + environment variable
# Config: screenshots_dir = "~/Documents"
# Environment: HYPRSHOT_DIR=~/Desktop
hyprshot-rs -m region
# Result: Saves to ~/Desktop (env overrides config)

# Scenario 3: Config + environment + CLI
# Config: screenshots_dir = "~/Documents"
# Environment: HYPRSHOT_DIR=~/Desktop
hyprshot-rs -m region -o /tmp
# Result: Saves to /tmp (CLI overrides everything)

# Scenario 4: Nothing configured
hyprshot-rs -m region
# Result: Saves to ~/Pictures (default)
```

---

## Managing Configuration

### Initialization

Create default configuration file:

```bash
hyprshot-rs --init-config
```

This will:
- Create `~/.config/hyprshot-rs/config.toml`
- Populate with default values
- Show confirmation message
- Provide usage examples

### Viewing Configuration

Show complete configuration:

```bash
hyprshot-rs --show-config
```

Output:
```
Configuration file: /home/username/.config/hyprshot-rs/config.toml

[paths]
screenshots_dir = "/home/username/Pictures"

[hotkeys]
window = "SUPER, Print"
...
```

Get config file path:

```bash
hyprshot-rs --config-path
```

Output:
```
/home/username/.config/hyprshot-rs/config.toml
```

### Modifying Configuration

#### Using CLI Commands

Set individual values:

```bash
# Set screenshots directory
hyprshot-rs --set paths.screenshots_dir ~/Screenshots

# Enable clipboard on capture
hyprshot-rs --set capture.clipboard_on_capture true

# Set notification timeout
hyprshot-rs --set capture.notification_timeout 5000

# Change hotkey
hyprshot-rs --set hotkeys.window "ALT, Print"
```

#### Manual Editing

Edit the TOML file directly:

```bash
# Using default editor
$EDITOR ~/.config/hyprshot-rs/config.toml

# Using specific editor
nano ~/.config/hyprshot-rs/config.toml
vim ~/.config/hyprshot-rs/config.toml
code ~/.config/hyprshot-rs/config.toml
```

#### Validation

Configuration is validated:
- When using `--set` (immediate validation)
- When loading config (at runtime)
- Invalid values are rejected with clear error messages

### Resetting Configuration

Delete and recreate:

```bash
# Remove config file
rm ~/.config/hyprshot-rs/config.toml

# Create fresh default config
hyprshot-rs --init-config
```

### Backup and Restore

Backup your configuration:

```bash
# Create backup
cp ~/.config/hyprshot-rs/config.toml ~/.config/hyprshot-rs/config.toml.backup

# Create timestamped backup
cp ~/.config/hyprshot-rs/config.toml \
   ~/.config/hyprshot-rs/config.toml.$(date +%Y%m%d-%H%M%S)
```

Restore from backup:

```bash
# Restore backup
cp ~/.config/hyprshot-rs/config.toml.backup \
   ~/.config/hyprshot-rs/config.toml
```

---

## Configuration Examples

### Minimal Configuration

Just set the screenshots directory:

```toml
[paths]
screenshots_dir = "~/Screenshots"

[hotkeys]
window = "SUPER, Print"
region = "SUPER SHIFT, Print"
output = "SUPER CTRL, Print"
active_output = ", Print"

[capture]
default_format = "png"
clipboard_on_capture = false
notification = true
notification_timeout = 3000

[advanced]
freeze_on_region = true
delay_ms = 0
```

### Power User Configuration

Customized for productivity:

```toml
[paths]
# Organized screenshots folder
screenshots_dir = "~/Documents/Screenshots"

[hotkeys]
# Custom hotkeys (Alt-based)
window = "ALT, Print"
region = "ALT SHIFT, Print"
output = "ALT CTRL, Print"
active_output = "ALT SUPER, Print"

[capture]
default_format = "png"
# Always copy to clipboard
clipboard_on_capture = true
notification = true
# Quick notifications
notification_timeout = 2000

[advanced]
# Freeze for precision
freeze_on_region = true
# Small delay for UI cleanup
delay_ms = 500
```

### Work Environment Configuration

Professional settings:

```toml
[paths]
# Work screenshots folder
screenshots_dir = "$HOME/Work/Screenshots"

[hotkeys]
window = "SUPER, F9"
region = "SUPER, F10"
output = "SUPER, F11"
active_output = "SUPER, F12"

[capture]
default_format = "png"
clipboard_on_capture = true
# Silent mode for meetings
notification = false
notification_timeout = 0

[advanced]
# No freeze for live demos
freeze_on_region = false
delay_ms = 0
```

### Content Creator Configuration

For tutorials and documentation:

```toml
[paths]
# Organized by project
screenshots_dir = "$PROJECT_DIR/assets/screenshots"

[hotkeys]
window = "SUPER, Print"
region = "SUPER SHIFT, Print"
output = "SUPER CTRL, Print"
active_output = ", Print"

[capture]
default_format = "png"
# Always ready to paste
clipboard_on_capture = true
notification = true
# Longer timeout to review
notification_timeout = 5000

[advanced]
# Freeze for clean captures
freeze_on_region = true
# Time to hide cursor/UI
delay_ms = 1000
```

### Streaming Configuration

Minimal disruption:

```toml
[paths]
screenshots_dir = "/tmp/stream-screenshots"

[hotkeys]
window = "CTRL ALT, F1"
region = "CTRL ALT, F2"
output = "CTRL ALT, F3"
active_output = "CTRL ALT, F4"

[capture]
default_format = "png"
clipboard_on_capture = false
# Silent mode (no notifications during stream)
notification = false
notification_timeout = 0

[advanced]
# No freeze (avoid screen flash)
freeze_on_region = false
delay_ms = 0
```

### Multi-Monitor Setup

For complex desktop environments:

```toml
[paths]
# Centralized location
screenshots_dir = "~/Pictures/Screenshots"

[hotkeys]
# Distinct hotkeys for each mode
window = "SUPER, KP_1"        # Numpad 1
region = "SUPER, KP_2"        # Numpad 2
output = "SUPER, KP_3"        # Numpad 3
active_output = "SUPER, KP_0" # Numpad 0

[capture]
default_format = "png"
clipboard_on_capture = true
notification = true
notification_timeout = 4000

[advanced]
freeze_on_region = true
# Slight delay for multi-monitor cursor positioning
delay_ms = 200
```

---

## Migration Guide

### From hyprshot (Shell Script)

If you're migrating from the original hyprshot shell script:

**Old (shell script):**
```bash
# Screenshots saved to ~/Pictures by default
# or configured via $HYPRSHOT_DIR
export HYPRSHOT_DIR=~/Screenshots
```

**New (hyprshot-rs):**
```bash
# Initialize config
hyprshot-rs --init-config

# Set directory
hyprshot-rs --set paths.screenshots_dir ~/Screenshots

# Or continue using environment variable
export HYPRSHOT_DIR=~/Screenshots
```

### From Manual Configuration

If you've been using CLI flags:

**Old approach:**
```bash
# Always specify location
hyprshot-rs -m region -o ~/Screenshots
```

**New approach:**
```bash
# Set once in config
hyprshot-rs --set paths.screenshots_dir ~/Screenshots

# Use without flags
hyprshot-rs -m region
```

---

## Troubleshooting

### Config File Not Found

**Problem:** Configuration commands don't work.

**Solution:**
```bash
# Check if config exists
hyprshot-rs --config-path

# Initialize if missing
hyprshot-rs --init-config
```

### Permission Denied

**Problem:** Can't save screenshots to configured directory.

**Solution:**
```bash
# Check directory permissions
ls -la ~/Pictures

# Create directory manually
mkdir -p ~/Pictures
chmod 755 ~/Pictures

# Or change to accessible location
hyprshot-rs --set paths.screenshots_dir ~/Documents
```

### Path Not Expanding

**Problem:** Paths like `~/Pictures` not expanding correctly.

**Solution:**
```bash
# Check expanded path
hyprshot-rs --show-config

# Path should show full path:
# screenshots_dir = "/home/username/Pictures"

# If not, try absolute path
hyprshot-rs --set paths.screenshots_dir /home/$USER/Pictures
```

### Environment Variable Not Working

**Problem:** `$XDG_PICTURES_DIR` or custom variables not expanding.

**Solution:**
```bash
# Check if variable is defined
echo $XDG_PICTURES_DIR

# If undefined, set it
export XDG_PICTURES_DIR="$HOME/Pictures"

# Or use a defined variable
hyprshot-rs --set paths.screenshots_dir '$HOME/Pictures'
```

### Invalid Configuration

**Problem:** Error messages about invalid config.

**Solution:**
```bash
# Backup current config
cp ~/.config/hyprshot-rs/config.toml ~/.config/hyprshot-rs/config.toml.backup

# Reinitialize with defaults
rm ~/.config/hyprshot-rs/config.toml
hyprshot-rs --init-config

# Manually restore needed settings
hyprshot-rs --set paths.screenshots_dir ~/Screenshots
```

### Config Changes Not Taking Effect

**Problem:** Changes to config file not working.

**Solution:**
```bash
# Verify config syntax (TOML format)
cat ~/.config/hyprshot-rs/config.toml

# Check for syntax errors (strings need quotes)
# WRONG: screenshots_dir = ~/Pictures
# RIGHT: screenshots_dir = "~/Pictures"

# Use CLI to set values (auto-formats correctly)
hyprshot-rs --set paths.screenshots_dir ~/Pictures
```

### Hotkeys Not Working

**Problem:** Keybindings in config don't work.

**Solution:**

The `[hotkeys]` section is informational only. You must add bindings to Hyprland config:

```bash
# Edit Hyprland config
nano ~/.config/hypr/hyprland.conf

# Add bindings:
bind = SUPER, Print, exec, hyprshot-rs -m window
bind = SUPER SHIFT, Print, exec, hyprshot-rs -m region
bind = SUPER CTRL, Print, exec, hyprshot-rs -m output

# Reload Hyprland config
hyprctl reload
```

---

## See Also

- [CLI Reference](CLI.md) - Complete command-line interface documentation
- [README.md](../README.md) - Project overview and installation guide
- [TODO.md](../TODO.md) - Development roadmap and features

---

**Configuration Version:** 1.0  
**Last Updated:** 2025  
**hyprshot-rs Version:** 0.1.3
