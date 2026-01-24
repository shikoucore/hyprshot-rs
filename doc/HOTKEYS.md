# Hotkeys Guide - hyprshot-rs

Complete guide for setting up keybindings for hyprshot-rs in various window managers and compositors.

## Table of Contents

- [Quick Start](#quick-start)
- [Hyprland](#hyprland)
- [Sway](#sway)
- [i3](#i3)
- [Customizing Hotkeys](#customizing-hotkeys)
- [Common Patterns](#common-patterns)
- [Troubleshooting](#troubleshooting)

---

## Quick Start

### For Hyprland Users

The easiest way to set up keybindings:

```bash
# Generate keybindings based on your config
hyprshot-rs --generate-hyprland-config --with-clipboard

# Automatically install to hyprland.conf (creates backup)
hyprshot-rs --install-binds --with-clipboard

# Reload Hyprland
hyprctl reload
```

### Manual Setup

1. Initialize config (if not done already):
   ```bash
   hyprshot-rs --init-config
   ```

2. Customize hotkeys in config:
   ```bash
   hyprshot-rs --set hotkeys.window "SUPER, Print"
   hyprshot-rs --set hotkeys.region "SUPER SHIFT, Print"
   ```

3. Generate and install bindings:
   ```bash
   hyprshot-rs --generate-hyprland-config
   ```

---

## Hyprland

### Automatic Installation

Generate and install keybindings automatically:

```bash
# Install basic screenshot bindings
hyprshot-rs --install-binds

# Install with clipboard-only variants (recommended)
hyprshot-rs --install-binds --with-clipboard
```

This will:
- Create a backup at `~/.config/hypr/hyprland.conf.backup`
- Append keybindings to your `hyprland.conf`
- Show you what was installed

To apply changes:
```bash
hyprctl reload
```

### Manual Installation

1. Generate keybindings:
   ```bash
   hyprshot-rs --generate-hyprland-config --with-clipboard
   ```

2. Copy the output and paste into `~/.config/hypr/hyprland.conf`

3. Reload Hyprland:
   ```bash
   hyprctl reload
   ```

### Default Keybindings

**Basic Screenshots:**
```conf
bind = SUPER, Print, exec, hyprshot-rs -m window
bind = SUPER SHIFT, Print, exec, hyprshot-rs -m region
bind = SUPER CTRL, Print, exec, hyprshot-rs -m output
bind = , Print, exec, hyprshot-rs -m output -m active
```

**Clipboard-Only Screenshots** (with `--with-clipboard`):
```conf
bind = SUPER ALT, Print, exec, hyprshot-rs -m window --clipboard-only
bind = SUPER SHIFT ALT, Print, exec, hyprshot-rs -m region --clipboard-only
bind = SUPER CTRL ALT, Print, exec, hyprshot-rs -m output --clipboard-only
```

### Custom Keybindings

Edit your config to use different keys:

```bash
# Use F keys instead of Print
hyprshot-rs --set hotkeys.window "SUPER, F9"
hyprshot-rs --set hotkeys.region "SUPER, F10"
hyprshot-rs --set hotkeys.output "SUPER, F11"

# Regenerate bindings
hyprshot-rs --generate-hyprland-config
```

Example custom bindings:
```conf
# Using F keys
bind = SUPER, F9, exec, hyprshot-rs -m window
bind = SUPER, F10, exec, hyprshot-rs -m region
bind = SUPER, F11, exec, hyprshot-rs -m output

# Using S key (like Flameshot)
bind = SUPER, S, exec, hyprshot-rs -m region

# Using letter keys
bind = SUPER, W, exec, hyprshot-rs -m window    # Window
bind = SUPER, R, exec, hyprshot-rs -m region    # Region
bind = SUPER, O, exec, hyprshot-rs -m output    # Output
```

---

## Sway

### Configuration File

Add to `~/.config/sway/config`:

```conf
# Screenshot keybindings
bindsym $mod+Print exec hyprshot-rs -m window
bindsym $mod+Shift+Print exec hyprshot-rs -m region
bindsym $mod+Ctrl+Print exec hyprshot-rs -m output
bindsym Print exec hyprshot-rs -m output -m active

# Clipboard-only variants
bindsym $mod+Mod1+Print exec hyprshot-rs -m window --clipboard-only
bindsym $mod+Shift+Mod1+Print exec hyprshot-rs -m region --clipboard-only
bindsym $mod+Ctrl+Mod1+Print exec hyprshot-rs -m output --clipboard-only
```

Note: `$mod` is usually `Mod4` (Super/Windows key). `Mod1` is Alt.

### Alternative: Using Mode

For more complex setups, use a Sway mode:

```conf
# Enter screenshot mode
bindsym $mod+Print mode "screenshot"

mode "screenshot" {
    # Take screenshots
    bindsym w exec hyprshot-rs -m window; mode "default"
    bindsym r exec hyprshot-rs -m region; mode "default"
    bindsym o exec hyprshot-rs -m output; mode "default"
    bindsym a exec hyprshot-rs -m output -m active; mode "default"
    
    # Clipboard variants
    bindsym Shift+w exec hyprshot-rs -m window --clipboard-only; mode "default"
    bindsym Shift+r exec hyprshot-rs -m region --clipboard-only; mode "default"
    bindsym Shift+o exec hyprshot-rs -m output --clipboard-only; mode "default"
    
    # Exit mode
    bindsym Escape mode "default"
    bindsym Return mode "default"
}
```

Reload Sway:
```bash
swaymsg reload
```

---

## i3

### Configuration File

Add to `~/.config/i3/config`:

```conf
# Screenshot keybindings
bindsym $mod+Print exec --no-startup-id hyprshot-rs -m window
bindsym $mod+Shift+Print exec --no-startup-id hyprshot-rs -m region
bindsym $mod+Ctrl+Print exec --no-startup-id hyprshot-rs -m output
bindsym Print exec --no-startup-id hyprshot-rs -m output -m active

# Clipboard-only variants
bindsym $mod+Mod1+Print exec --no-startup-id hyprshot-rs -m window --clipboard-only
bindsym $mod+Shift+Mod1+Print exec --no-startup-id hyprshot-rs -m region --clipboard-only
bindsym $mod+Ctrl+Mod1+Print exec --no-startup-id hyprshot-rs -m output --clipboard-only
```

**Note:** hyprshot-rs is designed for Wayland compositors (Hyprland, Sway). For X11 window managers like i3, you might need to use alternative screenshot tools like `scrot`, `maim`, or `flameshot`.

Reload i3:
```bash
i3-msg reload
```

---

## Customizing Hotkeys

### Configuration File

Hotkeys are stored in `~/.config/hyprshot-rs/config.toml`:

```toml
[hotkeys]
window = "SUPER, Print"
region = "SUPER SHIFT, Print"
output = "SUPER CTRL, Print"
active_output = ", Print"
```

### Using CLI

Change hotkeys via command line:

```bash
# Change window hotkey to Alt+Print
hyprshot-rs --set hotkeys.window "ALT, Print"

# Change region to Super+S
hyprshot-rs --set hotkeys.region "SUPER, S"

# Change output to Super+F12
hyprshot-rs --set hotkeys.output "SUPER, F12"

# Use just Print key for active output
hyprshot-rs --set hotkeys.active_output ", Print"
```

### Regenerating Bindings

After changing hotkeys, regenerate bindings:

```bash
# Generate new bindings
hyprshot-rs --generate-hyprland-config --with-clipboard

# Or reinstall (remove old ones first)
hyprshot-rs --install-binds --with-clipboard
```

---

## Common Patterns

### Print Key Variants

```conf
# Print alone - active output
bind = , Print, exec, hyprshot-rs -m output -m active

# Super+Print - window
bind = SUPER, Print, exec, hyprshot-rs -m window

# Super+Shift+Print - region
bind = SUPER SHIFT, Print, exec, hyprshot-rs -m region

# Super+Ctrl+Print - full output
bind = SUPER CTRL, Print, exec, hyprshot-rs -m output
```

### Function Key Variants

```conf
bind = SUPER, F9, exec, hyprshot-rs -m window
bind = SUPER, F10, exec, hyprshot-rs -m region
bind = SUPER, F11, exec, hyprshot-rs -m output
bind = SUPER, F12, exec, hyprshot-rs -m output -m active
```

### Letter Key Shortcuts

```conf
# Quick access with Super+letter
bind = SUPER, S, exec, hyprshot-rs -m region    # S for Select
bind = SUPER, W, exec, hyprshot-rs -m window    # W for Window
bind = SUPER, O, exec, hyprshot-rs -m output    # O for Output
```

### Clipboard-First Pattern

```conf
# Default to clipboard (no file)
bind = SUPER, Print, exec, hyprshot-rs -m region --clipboard-only
bind = SUPER SHIFT, Print, exec, hyprshot-rs -m window --clipboard-only

# With Shift - save to file
bind = SUPER CTRL, Print, exec, hyprshot-rs -m region
bind = SUPER CTRL SHIFT, Print, exec, hyprshot-rs -m window
```

### Combined with Image Viewer

```conf
# Screenshot and immediately open in viewer
bind = SUPER, Print, exec, hyprshot-rs -m window -- eog
bind = SUPER SHIFT, Print, exec, hyprshot-rs -m region -- eog

# Screenshot and open in editor
bind = SUPER CTRL, Print, exec, hyprshot-rs -m region -- gimp
```

---

## Troubleshooting

### Keybindings Not Working

**Check if hyprshot-rs is installed:**
```bash
which hyprshot-rs
hyprshot-rs --help
```

**Check Hyprland config syntax:**
```bash
# Verify no syntax errors
hyprctl reload

# Check if bindings are loaded
hyprctl binds | grep hyprshot
```

**Test manually:**
```bash
# Try running directly
hyprshot-rs -m region

# Check if keybind executes
hyprctl dispatch exec "hyprshot-rs -m region"
```

### Conflicts with Other Bindings

**List all bindings:**
```bash
# Hyprland
hyprctl binds | grep Print

# Sway
swaymsg -t get_binding_modes
```

**Check for conflicts:**
If another program uses the same keybinding, you'll need to:
1. Change hyprshot-rs keybinding
2. Or disable the conflicting binding

**Common conflicts:**
- System screenshot tools (GNOME Screenshot, Spectacle)
- Screen recording tools (OBS, SimpleScreenRecorder)
- Other screenshot utilities (Flameshot, grim+slurp)

**Disable system screenshot tool:**
```bash
# Debian/Ubuntu
sudo systemctl mask --user org.gnome.SettingsDaemon.Screensaver

# Or just remove the keybinding in system settings
```

### Hyprland.conf Already Has hyprshot-rs Bindings

If you see this error when running `--install-binds`:
```
hyprshot-rs bindings already exist in hyprland.conf
Please remove them manually first
```

**Solution 1: Remove old bindings manually**
```bash
# Edit config
nano ~/.config/hypr/hyprland.conf

# Find and remove lines like:
# # hyprshot-rs keybindings
# bind = SUPER, Print, exec, hyprshot-rs -m window
# ...

# Then reinstall
hyprshot-rs --install-binds --with-clipboard
```

**Solution 2: Use generate instead**
```bash
# Just generate and copy manually
hyprshot-rs --generate-hyprland-config --with-clipboard
```

### Print Key Not Working

Some keyboards/laptops have Print as a special key that requires specific handling.

**Try alternative keys:**
```bash
# Use F12 instead
hyprshot-rs --set hotkeys.region "SUPER, F12"

# Or use letter keys
hyprshot-rs --set hotkeys.region "SUPER, S"
```

**Test Print key directly:**
```bash
# Use wev to see key events
wev

# Press Print key and check output
# Should show KEY_SYSRQ or KEY_PRINT
```

### Permission Denied

```
Error: Failed to write to hyprland.conf
```

**Check permissions:**
```bash
ls -la ~/.config/hypr/hyprland.conf
```

**Fix permissions:**
```bash
chmod 644 ~/.config/hypr/hyprland.conf
```

### Changes Not Applied

After installing bindings, changes don't work:

**Reload Hyprland:**
```bash
hyprctl reload
```

**Or restart Hyprland:**
- Log out and log back in
- Or: `hyprctl dispatch exit` and restart from TTY

---

## Advanced Usage

### Per-Monitor Screenshots

Capture specific monitors:

```conf
# Bind specific monitors to keys
bind = SUPER, KP_1, exec, hyprshot-rs -m output -m DP-1
bind = SUPER, KP_2, exec, hyprshot-rs -m output -m DP-2
bind = SUPER, KP_3, exec, hyprshot-rs -m output -m HDMI-A-1
```

List your monitors:
```bash
hyprctl monitors
```

### Screenshots with Delay

Useful for capturing menus, tooltips:

```conf
# 3 second delay
bind = SUPER SHIFT CTRL, Print, exec, hyprshot-rs -m region -D 3

# 5 second delay
bind = SUPER SHIFT ALT, Print, exec, hyprshot-rs -m window -D 5
```

### Custom Save Location

```conf
# Save to specific folder
bind = SUPER, Print, exec, hyprshot-rs -m region -o ~/Screenshots

# Save to Desktop
bind = SUPER SHIFT, Print, exec, hyprshot-rs -m window -o ~/Desktop

# Save to /tmp (temporary)
bind = SUPER CTRL, Print, exec, hyprshot-rs -m window -o /tmp
```

### Silent Screenshots

No notifications:

```conf
bind = SUPER SHIFT, Print, exec, hyprshot-rs -m region -s
```

---

## See Also

- [CLI Reference](CLI.md) - Complete command-line options
- [Configuration Guide](CONFIGURATION.md) - Full configuration reference
- [README.md](../README.md) - Project overview

---

**Last Updated:** 2025  
**hyprshot-rs Version:** 0.1.3
