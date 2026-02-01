# Configuration Guide - hyprshot-rs

Minimal configuration reference aligned with actual behavior.

## Overview

- Config is a TOML file.
- Priority: CLI args > `HYPRSHOT_DIR` env > config file > defaults.
- CLI config management is documented in `doc/CLI.md`.

## Configuration File Location

Default path:

```
~/.config/hyprshot-rs/config.toml
```

Get the active path:

```bash
hyprshot-rs --config-path
```

## Configuration Structure

```toml
[paths]
[hotkeys]
[capture]
[advanced]
```

### Default Configuration (current)

```toml
[paths]
screenshots_dir = "~/Pictures"

[hotkeys]
window = "SUPER, Print"
region = "SUPER SHIFT, Print"
output = "SUPER CTRL, Print"
active_output = ", Print"

[capture]
notification = true
notification_timeout = 3000

[advanced]
freeze_on_region = true
delay_ms = 0
```

## Section: Paths

### `screenshots_dir`

- Directory for saved screenshots.
- Used when `--clipboard-only` is not set.
- Created if missing; must be writable.

Path expansion:
- `~` and `$HOME` are expanded.
- `$XDG_PICTURES_DIR` is expanded if available.
- Other `$VAR` are expanded if set.
- Undefined variables are left as-is.
- Relative paths stay relative (no canonicalization).

Priority for save directory:
1. `-o/--output-folder`
2. `HYPRSHOT_DIR`
3. `paths.screenshots_dir`
4. `~/Pictures`

## Section: Hotkeys

These values are **only for Hyprland config generation and the hotkey wizard**.
They do not change runtime behavior by themselves.

For working examples, see `doc/HOTKEYS.md`.

## Section: Capture

### `notification`

- When `true`, a desktop notification is attempted after capture.
- Notification failures are logged but do not abort the capture.
- `--silent` forces notifications off.

### `notification_timeout`

- Timeout for notifications in milliseconds.

## Section: Advanced

### `freeze_on_region`

- Enables `--freeze` by default.
- Applies to **all capture modes**, not just region.
- If the compositor lacks required Wayland protocols, freeze is skipped with a warning.

### `delay_ms`

- Delay before capture in milliseconds.

## Managing Configuration

See `doc/CLI.md` for:
- `--init-config`
- `--show-config`
- `--config-path`
- `--set`
- `--no-config`
