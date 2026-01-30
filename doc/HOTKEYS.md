# Hotkeys Guide - hyprshot-rs

Minimal keybinding examples for supported compositors.

## Quick Start

For usage and flags, see [README.md](../README.md) and [CLI.md](CLI.md).

---

## Hyprland

Add to `~/.config/hypr/hyprland.conf`:

```conf
bind = SUPER, Print, exec, hyprshot-rs -m window
bind = SUPER SHIFT, Print, exec, hyprshot-rs -m region
bind = SUPER CTRL, Print, exec, hyprshot-rs -m output
bind = , Print, exec, hyprshot-rs -m output -m active
```

Reload Hyprland:
```bash
hyprctl reload
```

---

## Sway

Add to `~/.config/sway/config`:

```conf
bindsym $mod+Print exec hyprshot-rs -m window
bindsym $mod+Shift+Print exec hyprshot-rs -m region
bindsym $mod+Ctrl+Print exec hyprshot-rs -m output
bindsym Print exec hyprshot-rs -m output -m active
```

Reload Sway:
```bash
swaymsg reload
```
