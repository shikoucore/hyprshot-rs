use anyhow::{ Context, Result };
use directories::ProjectDirs;
use serde::{ Deserialize, Serialize };
use std::env;
use std::fs;
use std::path::PathBuf;

/// Main configuration structure for hyprshot-rs
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub paths: PathsConfig,
    #[serde(default)]
    pub hotkeys: HotkeysConfig,
    #[serde(default)]
    pub capture: CaptureConfig,
    #[serde(default)]
    pub advanced: AdvancedConfig,
}

/// Configuration for paths
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PathsConfig {
    /// Directory where screenshots will be saved
    /// Default: ~/Pictures
    #[serde(default = "default_screenshots_dir")]
    pub screenshots_dir: String,
}

/// Configuration for hotkeys (for Hyprland)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HotkeysConfig {
    /// Hotkey for window capture
    /// Default: "SUPER, Print"
    #[serde(default = "default_hotkey_window")]
    pub window: String,

    /// Hotkey for region capture
    /// Default: "SUPER SHIFT, Print"
    #[serde(default = "default_hotkey_region")]
    pub region: String,

    /// Hotkey for output (monitor) capture
    /// Default: "SUPER CTRL, Print"
    #[serde(default = "default_hotkey_output")]
    pub output: String,

    /// Hotkey for active output capture
    /// Default: ", Print"
    #[serde(default = "default_hotkey_active_output")]
    pub active_output: String,
}

/// Configuration for capture settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CaptureConfig {
    /// Default format for screenshots (png, jpeg, ppm)
    /// Default: "png"
    #[serde(default = "default_format")]
    pub default_format: String,

    /// Automatically copy screenshot to clipboard
    /// Default: false
    #[serde(default)]
    pub clipboard_on_capture: bool,

    /// Show notifications after capture
    /// Default: true
    #[serde(default = "default_notification")]
    pub notification: bool,

    /// Notification timeout in milliseconds
    /// Default: 3000
    #[serde(default = "default_notification_timeout")]
    pub notification_timeout: u32,
}

/// Advanced configuration options
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AdvancedConfig {
    /// Freeze screen when selecting region
    /// Default: true
    #[serde(default = "default_freeze")]
    pub freeze_on_region: bool,

    /// Delay before capture in milliseconds
    /// Default: 0
    #[serde(default)]
    pub delay_ms: u32,
}

// Default value functions for serde
fn default_screenshots_dir() -> String {
    "~/Pictures".to_string()
}

fn default_hotkey_window() -> String {
    "SUPER, Print".to_string()
}

fn default_hotkey_region() -> String {
    "SUPER SHIFT, Print".to_string()
}

fn default_hotkey_output() -> String {
    "SUPER CTRL, Print".to_string()
}

fn default_hotkey_active_output() -> String {
    ", Print".to_string()
}

fn default_format() -> String {
    "png".to_string()
}

fn default_notification() -> bool {
    true
}

fn default_notification_timeout() -> u32 {
    3000
}

fn default_freeze() -> bool {
    true
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            screenshots_dir: default_screenshots_dir(),
        }
    }
}

impl Default for HotkeysConfig {
    fn default() -> Self {
        Self {
            window: default_hotkey_window(),
            region: default_hotkey_region(),
            output: default_hotkey_output(),
            active_output: default_hotkey_active_output(),
        }
    }
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            default_format: default_format(),
            clipboard_on_capture: false,
            notification: default_notification(),
            notification_timeout: default_notification_timeout(),
        }
    }
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            freeze_on_region: default_freeze(),
            delay_ms: 0,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            paths: PathsConfig::default(),
            hotkeys: HotkeysConfig::default(),
            capture: CaptureConfig::default(),
            advanced: AdvancedConfig::default(),
        }
    }
}

// Utility functions for path expansion and validation

/// Expand path with support for:
/// - `~` → home directory
/// - `$HOME` → home directory
/// - `$XDG_PICTURES_DIR` → Pictures directory from environment or XDG config
/// - Other `$VAR` → environment variables
pub fn expand_path(path: &str) -> Result<PathBuf> {
    let path = path.trim();

    // Handle empty path
    if path.is_empty() {
        return Ok(PathBuf::from("."));
    }

    // Expand ~ at the beginning
    let path = if path.starts_with("~/") || path == "~" {
        let home = dirs::home_dir().context("Failed to get home directory")?;
        if path == "~" {
            home
        } else {
            home.join(&path[2..])
        }
    } else {
        PathBuf::from(path)
    };

    let path_str = path.to_string_lossy();
    let mut result = String::new();
    let mut chars = path_str.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            let mut var_name = String::new();
            while let Some(&next_ch) = chars.peek() {
                if next_ch.is_alphanumeric() || next_ch == '_' {
                    var_name.push(chars.next().unwrap());
                } else {
                    break;
                }
            }

            if var_name == "XDG_PICTURES_DIR" {
                if let Some(pictures_dir) = dirs::picture_dir() {
                    result.push_str(&pictures_dir.to_string_lossy());
                } else {
                    // $HOME/Pictures
                    if let Some(home) = dirs::home_dir() {
                        result.push_str(&home.join("Pictures").to_string_lossy());
                    } else {
                        result.push_str("Pictures");
                    }
                }
            } else if !var_name.is_empty() {
                if let Ok(value) = env::var(&var_name) {
                    result.push_str(&value);
                } else {
                    // original $VAR if not found
                    result.push('$');
                    result.push_str(&var_name);
                }
            } else {
                result.push('$');
            }
        } else {
            result.push(ch);
        }
    }

    Ok(PathBuf::from(result))
}

/// Validate and prepare directory for saving screenshots
/// - Expands path variables
/// - Creates directory if it doesn't exist
/// - Returns error if path is not writable
pub fn ensure_directory(path: &str) -> Result<PathBuf> {
    let expanded_path = expand_path(path)?;

    if !expanded_path.exists() {
        fs
            ::create_dir_all(&expanded_path)
            .context(format!("Failed to create directory: {}", expanded_path.display()))?;
    }

    if !expanded_path.is_dir() {
        return Err(
            anyhow::anyhow!("Path exists but is not a directory: {}", expanded_path.display())
        );
    }

    let test_file = expanded_path.join(".hyprshot_test");
    match fs::write(&test_file, b"test") {
        Ok(_) => {
            let _ = fs::remove_file(&test_file);
            Ok(expanded_path)
        }
        Err(e) => {
            Err(anyhow::anyhow!("Directory is not writable: {} - {}", expanded_path.display(), e))
        }
    }
}

/// Get screenshot save directory with priority:
/// 1. CLI argument (if provided)
/// 2. Environment variable HYPRSHOT_DIR
/// 3. Config file value
/// 4. Default ~/Pictures
pub fn get_screenshots_dir(
    cli_path: Option<PathBuf>,
    config: &Config,
    debug: bool
) -> Result<PathBuf> {
    if let Some(path) = cli_path {
        if debug {
            eprintln!("Using screenshot directory from CLI: {}", path.display());
        }
        return Ok(path);
    }

    if let Ok(env_path) = env::var("HYPRSHOT_DIR") {
        let expanded = expand_path(&env_path)?;
        if debug {
            eprintln!("Using screenshot directory from HYPRSHOT_DIR: {}", expanded.display());
        }
        return Ok(expanded);
    }

    let config_path = expand_path(&config.paths.screenshots_dir)?;
    if debug {
        eprintln!("Using screenshot directory from config: {}", config_path.display());
    }
    Ok(config_path)
}

impl Config {
    /// Get the path to the configuration file
    /// Returns ~/.config/hyprshot-rs/config.toml
    pub fn config_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("", "", "hyprshot-rs").context(
            "Failed to determine config directory"
        )?;

        let config_dir = proj_dirs.config_dir();
        Ok(config_dir.join("config.toml"))
    }

    /// Get the configuration directory
    /// Returns ~/.config/hyprshot-rs/
    pub fn config_dir() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("", "", "hyprshot-rs").context(
            "Failed to determine config directory"
        )?;

        Ok(proj_dirs.config_dir().to_path_buf())
    }

    /// Load configuration from file
    /// If file doesn't exist, returns default configuration
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            // Config doesn't exist, return default
            return Ok(Self::default());
        }

        let content = fs
            ::read_to_string(&config_path)
            .context(format!("Failed to read config file: {}", config_path.display()))?;

        let config: Config = toml
            ::from_str(&content)
            .context("Failed to parse config file. Check TOML syntax.")?;

        Ok(config)
    }

    /// Save configuration to file
    /// Creates config directory if it doesn't exist
    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        let config_path = Self::config_path()?;

        if !config_dir.exists() {
            fs
                ::create_dir_all(&config_dir)
                .context(format!("Failed to create config directory: {}", config_dir.display()))?;
        }

        let toml_string = toml
            ::to_string_pretty(self)
            .context("Failed to serialize config to TOML")?;

        let commented_toml = Self::add_comments(&toml_string);

        fs
            ::write(&config_path, commented_toml)
            .context(format!("Failed to write config file: {}", config_path.display()))?;

        Ok(())
    }

    /// Initialize config with default values and save to file
    /// This creates the config directory and file if they don't exist
    pub fn init() -> Result<Self> {
        let config = Self::default();
        config.save()?;
        Ok(config)
    }

    /// Check if config file exists
    pub fn exists() -> bool {
        Self::config_path()
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    /// Add helpful comments to the TOML configuration
    fn add_comments(toml: &str) -> String {
        let header =
            "# hyprshot-rs configuration file\n\
                      # This file is automatically generated. Edit with care.\n\
                      # For more information, see: https://github.com/vremyavnikuda/hyprshot-rs\n\n";

        let mut result = String::from(header);

        for line in toml.lines() {
            // Add section comments
            if line.starts_with("[paths]") {
                result.push_str("# Paths configuration\n");
            } else if line.starts_with("[hotkeys]") {
                result.push_str("\n# Hotkeys configuration for Hyprland\n");
                result.push_str("# Format: \"MODIFIER, KEY\"\n");
                result.push_str("# Examples: \"SUPER, Print\", \"SUPER SHIFT, S\", \", Print\"\n");
            } else if line.starts_with("[capture]") {
                result.push_str("\n# Capture settings\n");
            } else if line.starts_with("[advanced]") {
                result.push_str("\n# Advanced settings\n");
            }

            result.push_str(line);
            result.push('\n');
        }

        result
    }

    /// Generate Hyprland keybindings based on config
    /// Returns a String with bind statements ready to paste into hyprland.conf
    pub fn generate_hyprland_binds(&self) -> String {
        let mut binds = String::new();

        binds.push_str("# hyprshot-rs keybindings\n");
        binds.push_str("# Generated by: hyprshot-rs --generate-hyprland-config\n\n");

        // Basic screenshot bindings
        binds.push_str("# Screenshot keybindings\n");
        binds.push_str(&format!("bind = {}, exec, hyprshot-rs -m window\n", self.hotkeys.window));
        binds.push_str(&format!("bind = {}, exec, hyprshot-rs -m region\n", self.hotkeys.region));
        binds.push_str(&format!("bind = {}, exec, hyprshot-rs -m output\n", self.hotkeys.output));
        binds.push_str(
            &format!(
                "bind = {}, exec, hyprshot-rs -m active -m output\n",
                self.hotkeys.active_output
            )
        );

        binds
    }

    /// Generate Hyprland keybindings with clipboard-only variants
    /// Adds additional bindings with ALT modifier for clipboard-only mode
    pub fn generate_hyprland_binds_with_clipboard(&self) -> String {
        let mut binds = self.generate_hyprland_binds();

        binds.push_str("\n# Screenshot to clipboard (no file saved)\n");

        let window_clipboard = self.add_alt_modifier(&self.hotkeys.window);
        let region_clipboard = self.add_alt_modifier(&self.hotkeys.region);
        let output_clipboard = self.add_alt_modifier(&self.hotkeys.output);

        binds.push_str(
            &format!("bind = {}, exec, hyprshot-rs -m window --clipboard-only\n", window_clipboard)
        );
        binds.push_str(
            &format!("bind = {}, exec, hyprshot-rs -m region --clipboard-only\n", region_clipboard)
        );
        binds.push_str(
            &format!("bind = {}, exec, hyprshot-rs -m output --clipboard-only\n", output_clipboard)
        );

        binds
    }

    /// Add ALT modifier to a hotkey string
    /// Examples:
    ///   "SUPER, Print" -> "SUPER ALT, Print"
    ///   ", Print" -> "ALT, Print"
    ///   "CTRL, S" -> "CTRL ALT, S"
    fn add_alt_modifier(&self, hotkey: &str) -> String {
        if let Some((modifiers, key)) = hotkey.split_once(',') {
            let modifiers = modifiers.trim();
            let key = key.trim();

            if modifiers.is_empty() {
                format!("ALT, {}", key)
            } else if modifiers.contains("ALT") {
                hotkey.to_string()
            } else {
                format!("{} ALT, {}", modifiers, key)
            }
        } else {
            hotkey.to_string()
        }
    }

    /// Install Hyprland bindings to hyprland.conf
    /// Returns the path where bindings were installed
    pub fn install_hyprland_binds(&self, with_clipboard: bool) -> Result<PathBuf> {
        let hyprland_conf = dirs
            ::home_dir()
            .context("Failed to get home directory")?
            .join(".config/hypr/hyprland.conf");

        if !hyprland_conf.exists() {
            anyhow::bail!(
                "Hyprland config not found at: {}\nPlease create it first or check your Hyprland installation.",
                hyprland_conf.display()
            );
        }

        let existing_config = fs
            ::read_to_string(&hyprland_conf)
            .context("Failed to read hyprland.conf")?;

        if existing_config.contains("# hyprshot-rs keybindings") {
            anyhow::bail!(
                "hyprshot-rs bindings already exist in hyprland.conf\n\
                Please remove them manually first, or use --generate-hyprland-config to print bindings."
            );
        }

        let binds = if with_clipboard {
            self.generate_hyprland_binds_with_clipboard()
        } else {
            self.generate_hyprland_binds()
        };

        let mut new_config = existing_config;
        if !new_config.ends_with('\n') {
            new_config.push('\n');
        }
        new_config.push('\n');
        new_config.push_str(&binds);

        let backup_path = hyprland_conf.with_extension("conf.backup");
        fs::copy(&hyprland_conf, &backup_path).context("Failed to create backup of hyprland.conf")?;

        fs::write(&hyprland_conf, new_config).context("Failed to write to hyprland.conf")?;

        Ok(hyprland_conf)
    }

    /// Get the path to Hyprland config file
    pub fn hyprland_config_path() -> Result<PathBuf> {
        let path = dirs
            ::home_dir()
            .context("Failed to get home directory")?
            .join(".config/hypr/hyprland.conf");
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.paths.screenshots_dir, "~/Pictures");
        assert_eq!(config.hotkeys.window, "SUPER, Print");
        assert_eq!(config.capture.default_format, "png");
        assert_eq!(config.capture.notification, true);
        assert_eq!(config.capture.notification_timeout, 3000);
        assert_eq!(config.advanced.freeze_on_region, true);
        assert_eq!(config.advanced.delay_ms, 0);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("[paths]"));
        assert!(toml_str.contains("[hotkeys]"));
        assert!(toml_str.contains("[capture]"));
        assert!(toml_str.contains("[advanced]"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str =
            r#"
            [paths]
            screenshots_dir = "~/Documents"

            [hotkeys]
            window = "ALT, W"
            region = "ALT, R"

            [capture]
            default_format = "jpeg"
            notification = false

            [advanced]
            delay_ms = 500
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.paths.screenshots_dir, "~/Documents");
        assert_eq!(config.hotkeys.window, "ALT, W");
        assert_eq!(config.capture.default_format, "jpeg");
        assert_eq!(config.capture.notification, false);
        assert_eq!(config.advanced.delay_ms, 500);
    }

    #[test]
    fn test_partial_config() {
        let toml_str = r#"
            [paths]
            screenshots_dir = "~/Custom"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.paths.screenshots_dir, "~/Custom");
        // These should have default values
        assert_eq!(config.hotkeys.window, "SUPER, Print");
        assert_eq!(config.capture.notification, true);
    }

    #[test]
    fn test_expand_path_tilde() {
        let result = expand_path("~/Pictures").unwrap();
        let home = dirs::home_dir().unwrap();
        assert_eq!(result, home.join("Pictures"));

        let result = expand_path("~").unwrap();
        assert_eq!(result, home);
    }

    #[test]
    fn test_expand_path_env_vars() {
        unsafe {
            env::set_var("TEST_VAR", "/test/path");
        }

        let result = expand_path("$TEST_VAR/screenshots").unwrap();
        assert_eq!(result, PathBuf::from("/test/path/screenshots"));

        unsafe {
            env::remove_var("TEST_VAR");
        }
    }

    #[test]
    fn test_expand_path_xdg_pictures() {
        // $XDG_PICTURES_DIR
        let result = expand_path("$XDG_PICTURES_DIR/screenshots").unwrap();
        if let Some(pictures_dir) = dirs::picture_dir() {
            assert_eq!(result, pictures_dir.join("screenshots"));
        } else {
            let home = dirs::home_dir().unwrap();
            assert_eq!(result, home.join("Pictures/screenshots"));
        }
    }

    #[test]
    fn test_expand_path_empty() {
        let result = expand_path("").unwrap();
        assert_eq!(result, PathBuf::from("."));
    }

    #[test]
    fn test_expand_path_no_expansion() {
        let result = expand_path("/absolute/path").unwrap();
        assert_eq!(result, PathBuf::from("/absolute/path"));

        let result = expand_path("relative/path").unwrap();
        assert_eq!(result, PathBuf::from("relative/path"));
    }

    #[test]
    fn test_expand_path_undefined_var() {
        let result = expand_path("$UNDEFINED_VAR_12345/test").unwrap();
        assert_eq!(result, PathBuf::from("$UNDEFINED_VAR_12345/test"));
    }

    #[test]
    fn test_get_screenshots_dir_priority_cli() {
        let config = Config::default();
        let cli_path = Some(PathBuf::from("/cli/path"));

        unsafe {
            env::set_var("HYPRSHOT_DIR", "/env/path");
        }

        let result = get_screenshots_dir(cli_path, &config, false).unwrap();
        assert_eq!(result, PathBuf::from("/cli/path"));

        unsafe {
            env::remove_var("HYPRSHOT_DIR");
        }
    }

    #[test]
    fn test_get_screenshots_dir_priority_env() {
        let config = Config::default();

        unsafe {
            env::set_var("HYPRSHOT_DIR", "/env/path");
        }

        let result = get_screenshots_dir(None, &config, false).unwrap();
        assert_eq!(result, PathBuf::from("/env/path"));

        unsafe {
            env::remove_var("HYPRSHOT_DIR");
        }
    }

    #[test]
    fn test_get_screenshots_dir_priority_config() {
        let mut config = Config::default();
        config.paths.screenshots_dir = "/config/path".to_string();

        let result = get_screenshots_dir(None, &config, false).unwrap();
        assert_eq!(result, PathBuf::from("/config/path"));
    }

    #[test]
    fn test_get_screenshots_dir_with_tilde() {
        let mut config = Config::default();
        config.paths.screenshots_dir = "~/Screenshots".to_string();

        let result = get_screenshots_dir(None, &config, false).unwrap();
        let home = dirs::home_dir().unwrap();
        assert_eq!(result, home.join("Screenshots"));
    }

    #[test]
    fn test_generate_hyprland_binds() {
        let config = Config::default();
        let binds = config.generate_hyprland_binds();

        assert!(binds.contains("# hyprshot-rs keybindings"));
        assert!(binds.contains("# Generated by: hyprshot-rs --generate-hyprland-config"));

        assert!(binds.contains("bind = SUPER, Print, exec, hyprshot-rs -m window"));
        assert!(binds.contains("bind = SUPER SHIFT, Print, exec, hyprshot-rs -m region"));
        assert!(binds.contains("bind = SUPER CTRL, Print, exec, hyprshot-rs -m output"));
        assert!(binds.contains("bind = , Print, exec, hyprshot-rs -m active -m output"));

        assert!(!binds.contains("--clipboard-only"));
    }

    #[test]
    fn test_generate_hyprland_binds_with_clipboard() {
        let config = Config::default();
        let binds = config.generate_hyprland_binds_with_clipboard();

        assert!(binds.contains("bind = SUPER, Print, exec, hyprshot-rs -m window"));
        assert!(binds.contains("bind = SUPER SHIFT, Print, exec, hyprshot-rs -m region"));

        assert!(binds.contains("# Screenshot to clipboard (no file saved)"));
        assert!(
            binds.contains("bind = SUPER ALT, Print, exec, hyprshot-rs -m window --clipboard-only")
        );
        assert!(
            binds.contains(
                "bind = SUPER SHIFT ALT, Print, exec, hyprshot-rs -m region --clipboard-only"
            )
        );
        assert!(
            binds.contains(
                "bind = SUPER CTRL ALT, Print, exec, hyprshot-rs -m output --clipboard-only"
            )
        );
    }

    #[test]
    fn test_add_alt_modifier() {
        let config = Config::default();

        assert_eq!(config.add_alt_modifier("SUPER, Print"), "SUPER ALT, Print");

        assert_eq!(config.add_alt_modifier(", Print"), "ALT, Print");

        assert_eq!(config.add_alt_modifier("SUPER SHIFT, Print"), "SUPER SHIFT ALT, Print");

        assert_eq!(config.add_alt_modifier("SUPER ALT, Print"), "SUPER ALT, Print");
        assert_eq!(config.add_alt_modifier("ALT, Print"), "ALT, Print");

        assert_eq!(config.add_alt_modifier("CTRL, S"), "CTRL ALT, S");
    }
}
