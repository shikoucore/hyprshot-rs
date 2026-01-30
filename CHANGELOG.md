# Changelog

All notable changes to hyprshot-rs will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [release 0.1.6]

### Changed
- **Freeze implementation**: Replaced hyprpicker-based freeze with native Wayland layer-shell overlay.
- **Output-by-name (Wayland)**: `-m output -m NAME` now resolves outputs via Wayland enumeration (no hyprctl validation in CLI).
- **Freeze memory usage**: Capture is performed per-output to reduce peak RAM on multi-monitor setups.

### Fixed
- **Freeze portability**: Added geometry-based output matching fallback when output names are unavailable.
- **Freeze robustness**: Gracefully disables freeze if required Wayland protocols are missing (with clear user message).
- **Freeze input handling**: Overlay is input-transparent to avoid blocking selection.
- **Freeze scaling**: Better handling of fractional scaling and logical output sizes.
- **More reliable saves**: Clipboard/notification errors no longer break successful captures (except `--clipboard-only`).
- **Delay accuracy**: `delay_ms` now respects milliseconds instead of rounding to seconds.
- **Notification timeout**: `--notif-timeout` always respects the value you pass (including 5000).

## [release 0.1.5] 2026-01-29

### Fixed
- **CLI output names**: Allow OUTPUT_NAME in `-m` and correct help flag for `--notif-timeout`.

## [release 0.1.4] 2026-01-24

### Added
- **Embedded slurp build**: Build and embed slurp during compilation with automatic fallback to system slurp
- **AUR support**: Packaging support via the AUR submodule
- **Vendored slurp sources**: Include slurp sources in crate packaging to ensure crates.io builds

### Changed
- **grim-rs update**: Bumped grim-rs dependency to v0.1.4
- **Documentation**: Clarified mode combinations (`active` modifier), fixed CLI flags, and noted reserved config options

### Fixed
- **Embedded slurp include path**: Generate `include_bytes!` via `OUT_DIR` to avoid broken absolute paths

## [release 0.1.3] 2025-10-04

### Added
- **Configuration System**: Full TOML-based configuration with `~/.config/hyprshot-rs/config.toml`
  - Persistent settings for paths, hotkeys, capture options, and advanced settings
  - CLI commands: `--init-config`, `--show-config`, `--config-path`, `--set KEY VALUE`
  - Path expansion support (`~`, `$HOME`, `$VAR`, `$XDG_PICTURES_DIR`)
  - Priority system: CLI arguments > Environment variables > Config file > Defaults
  - Flag `--no-config` to ignore configuration file

- **Hyprland Integration**: Automatic keybinding generation and installation
  - `--generate-hyprland-config`: Generate keybindings from config
  - `--install-binds`: Install keybindings to hyprland.conf (with automatic backup)
  - `--with-clipboard`: Include clipboard-only variants with ALT modifier
  - `--setup-hotkeys`: Interactive wizard for hotkey configuration

- **Integrated Config Settings**: Settings now read from config with CLI override
  - `capture.notification` / `--silent`: Control notifications
  - `capture.notification_timeout` / `--notif-timeout`: Notification duration
  - `advanced.freeze_on_region` / `--freeze`: Freeze screen on region selection
  - `advanced.delay_ms` / `--delay`: Delay before capture
  - `paths.screenshots_dir` / `-o`: Screenshot save directory

- **Documentation**: Comprehensive guides added to `doc/`
  - `CLI.md`: Complete CLI reference (528 lines)
  - `CONFIGURATION.md`: Configuration guide with examples (1095 lines)
  - `HOTKEYS.md`: Hotkeys setup for Hyprland, Sway, i3 (530 lines)

### Dependencies
- Added `serde` with derive feature for config serialization
- Added `toml` for TOML parsing
- Added `directories` for XDG directory support
- Added `dialoguer` for interactive CLI wizards

## [release 0.1.2] 

### Changed
- **Replaced external grim dependency with grim-rs v0.1.2**: Integrated native Rust implementation of screenshot functionality
  - Removed dependency on external C-based grim utility
  - All screenshot capture now handled by embedded grim-rs library
  - Improved performance and reduced external dependencies
  - Better error handling and type safety with Rust implementation

### Removed
- External grim binary dependency from installation requirements
- System package dependency on grim for Arch Linux and other distributions

## Previous Releases

For previous release history, see git commit log.
