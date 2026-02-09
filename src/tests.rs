#[cfg(not(target_os = "windows"))]
mod non_windows {
    use crate::{Args, Mode, default_filename, resolve_delay, resolve_notif_timeout};
    use chrono::TimeZone;
    use clap::Parser;
    use std::str::FromStr;
    use std::time::Duration;
    use std::{env, path::PathBuf};

    #[test]
    fn parse_output_name_mode() {
        let args = Args::parse_from(["hyprshot-rs", "-m", "output", "-m", "DP-1"]);
        assert!(matches!(args.mode.get(0), Some(Mode::Output)));
        assert!(matches!(
            args.mode.get(1),
            Some(Mode::OutputName(name)) if name == "DP-1"
        ));
    }

    #[test]
    fn notif_timeout_cli_overrides_config() {
        let mut config = crate::config::Config::default();
        config.capture.notification_timeout = 7000;

        let args = Args::parse_from(["hyprshot-rs", "-m", "region", "--notif-timeout", "5000"]);

        assert_eq!(resolve_notif_timeout(&args, &config), 5000);
    }

    #[test]
    fn delay_uses_milliseconds_from_config() {
        let mut config = crate::config::Config::default();
        config.advanced.delay_ms = 250;

        let args = Args::parse_from(["hyprshot-rs", "-m", "region"]);
        assert_eq!(resolve_delay(&args, &config), Duration::from_millis(250));
    }

    #[test]
    fn filename_includes_milliseconds() {
        let now = match chrono::Local
            .timestamp_millis_opt(1_700_000_000_123)
            .single()
        {
            Some(v) => v,
            None => panic!("Failed to construct timestamp for test"),
        };
        let name = default_filename(now);
        assert!(name.ends_with("-123_hyprshot.png"));
    }

    #[test]
    fn filenames_differ_for_distinct_timestamps() {
        let first = match chrono::Local
            .timestamp_millis_opt(1_700_000_000_001)
            .single()
        {
            Some(v) => v,
            None => panic!("Failed to construct first timestamp for test"),
        };
        let second = match chrono::Local
            .timestamp_millis_opt(1_700_000_000_002)
            .single()
        {
            Some(v) => v,
            None => panic!("Failed to construct second timestamp for test"),
        };
        let a = default_filename(first);
        let b = default_filename(second);
        assert_ne!(a, b);
    }

    #[test]
    fn geometry_parses_and_validates() {
        let geometry = match crate::geometry::Geometry::from_str("10,20 300x400") {
            Ok(v) => v,
            Err(err) => panic!("Failed to parse geometry: {}", err),
        };
        assert_eq!(geometry.x, 10);
        assert_eq!(geometry.y, 20);
        assert_eq!(geometry.width, 300);
        assert_eq!(geometry.height, 400);

        assert!(crate::geometry::Geometry::from_str("10,20 0x400").is_err());
        assert!(crate::geometry::Geometry::from_str("10,20 -1x400").is_err());
        assert!(crate::geometry::Geometry::from_str("10,20 300x0").is_err());
    }

    #[test]
    fn parse_active_output_mode_combo() {
        let args = Args::parse_from(["hyprshot-rs", "-m", "output", "-m", "active"]);
        assert!(matches!(args.mode.get(0), Some(Mode::Output)));
        assert!(matches!(args.mode.get(1), Some(Mode::Active)));
    }

    #[test]
    fn test_default_config() {
        let config = crate::config::Config::default();
        assert_eq!(config.paths.screenshots_dir, "~/Pictures");
        assert_eq!(config.hotkeys.window, "SUPER, Print");
        assert!(config.capture.notification);
        assert_eq!(config.capture.notification_timeout, 3000);
        assert!(config.advanced.freeze_on_region);
        assert_eq!(config.advanced.delay_ms, 0);
    }

    #[test]
    fn test_config_serialization() {
        let config = crate::config::Config::default();
        let toml_str = match toml::to_string(&config) {
            Ok(v) => v,
            Err(err) => panic!("Failed to serialize config: {}", err),
        };
        assert!(toml_str.contains("[paths]"));
        assert!(toml_str.contains("[hotkeys]"));
        assert!(toml_str.contains("[capture]"));
        assert!(toml_str.contains("[advanced]"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = r#"
            [paths]
            screenshots_dir = "~/Documents"
    
            [hotkeys]
            window = "ALT, W"
            region = "ALT, R"
    
            [capture]
            notification = false
    
            [advanced]
            delay_ms = 500
        "#;

        let config: crate::config::Config = match toml::from_str(toml_str) {
            Ok(v) => v,
            Err(err) => panic!("Failed to deserialize config: {}", err),
        };
        assert_eq!(config.paths.screenshots_dir, "~/Documents");
        assert_eq!(config.hotkeys.window, "ALT, W");
        assert_eq!(config.capture.notification, false);
        assert_eq!(config.advanced.delay_ms, 500);
    }

    #[test]
    fn test_partial_config() {
        let toml_str = r#"
            [paths]
            screenshots_dir = "~/Custom"
        "#;

        let config: crate::config::Config = match toml::from_str(toml_str) {
            Ok(v) => v,
            Err(err) => panic!("Failed to deserialize partial config: {}", err),
        };
        assert_eq!(config.paths.screenshots_dir, "~/Custom");
        assert_eq!(config.hotkeys.window, "SUPER, Print");
        assert!(config.capture.notification);
    }

    #[test]
    fn test_expand_path_tilde() {
        let result = match crate::config::expand_path("~/Pictures") {
            Ok(v) => v,
            Err(err) => panic!("Failed to expand path: {}", err),
        };
        let home = match dirs::home_dir() {
            Some(v) => v,
            None => panic!("Failed to resolve home directory"),
        };
        assert_eq!(result, home.join("Pictures"));

        let result = match crate::config::expand_path("~") {
            Ok(v) => v,
            Err(err) => panic!("Failed to expand home path: {}", err),
        };
        assert_eq!(result, home);
    }

    #[test]
    fn test_expand_path_env_vars() {
        unsafe {
            env::set_var("TEST_VAR", "/test/path");
        }

        let result = match crate::config::expand_path("$TEST_VAR/screenshots") {
            Ok(v) => v,
            Err(err) => panic!("Failed to expand env path: {}", err),
        };
        assert_eq!(result, PathBuf::from("/test/path/screenshots"));

        unsafe {
            env::remove_var("TEST_VAR");
        }
    }

    #[test]
    fn test_expand_path_xdg_pictures() {
        let result = match crate::config::expand_path("$XDG_PICTURES_DIR/screenshots") {
            Ok(v) => v,
            Err(err) => panic!("Failed to expand XDG pictures path: {}", err),
        };
        if let Some(pictures_dir) = dirs::picture_dir() {
            assert_eq!(result, pictures_dir.join("screenshots"));
        } else {
            let home = match dirs::home_dir() {
                Some(v) => v,
                None => panic!("Failed to resolve home directory"),
            };
            assert_eq!(result, home.join("Pictures/screenshots"));
        }
    }

    #[test]
    fn test_expand_path_empty() {
        let result = match crate::config::expand_path("") {
            Ok(v) => v,
            Err(err) => panic!("Failed to expand empty path: {}", err),
        };
        assert_eq!(result, PathBuf::from("."));
    }

    #[test]
    fn test_expand_path_no_expansion() {
        let result = match crate::config::expand_path("/absolute/path") {
            Ok(v) => v,
            Err(err) => panic!("Failed to expand absolute path: {}", err),
        };
        assert_eq!(result, PathBuf::from("/absolute/path"));

        let result = match crate::config::expand_path("relative/path") {
            Ok(v) => v,
            Err(err) => panic!("Failed to expand relative path: {}", err),
        };
        assert_eq!(result, PathBuf::from("relative/path"));
    }

    #[test]
    fn test_expand_path_undefined_var() {
        let result = match crate::config::expand_path("$UNDEFINED_VAR_12345/test") {
            Ok(v) => v,
            Err(err) => panic!("Failed to expand undefined var path: {}", err),
        };
        assert_eq!(result, PathBuf::from("$UNDEFINED_VAR_12345/test"));
    }

    #[test]
    fn test_get_screenshots_dir_priority_cli() {
        let config = crate::config::Config::default();
        let cli_path = Some(PathBuf::from("/cli/path"));

        unsafe {
            env::set_var("HYPRSHOT_DIR", "/env/path");
        }

        let result = match crate::config::get_screenshots_dir(cli_path, &config, false) {
            Ok(v) => v,
            Err(err) => panic!("Failed to resolve screenshots dir (cli): {}", err),
        };
        assert_eq!(result, PathBuf::from("/cli/path"));

        unsafe {
            env::remove_var("HYPRSHOT_DIR");
        }
    }

    #[test]
    fn test_get_screenshots_dir_priority_env() {
        let config = crate::config::Config::default();

        unsafe {
            env::set_var("HYPRSHOT_DIR", "/env/path");
        }

        let result = match crate::config::get_screenshots_dir(None, &config, false) {
            Ok(v) => v,
            Err(err) => panic!("Failed to resolve screenshots dir (env): {}", err),
        };
        assert_eq!(result, PathBuf::from("/env/path"));

        unsafe {
            env::remove_var("HYPRSHOT_DIR");
        }
    }

    #[test]
    fn test_get_screenshots_dir_priority_config() {
        let mut config = crate::config::Config::default();
        config.paths.screenshots_dir = "/config/path".to_string();

        let result = match crate::config::get_screenshots_dir(None, &config, false) {
            Ok(v) => v,
            Err(err) => panic!("Failed to resolve screenshots dir (config): {}", err),
        };
        assert_eq!(result, PathBuf::from("/config/path"));
    }

    #[test]
    fn test_get_screenshots_dir_with_tilde() {
        let mut config = crate::config::Config::default();
        config.paths.screenshots_dir = "~/Screenshots".to_string();

        let result = match crate::config::get_screenshots_dir(None, &config, false) {
            Ok(v) => v,
            Err(err) => panic!("Failed to resolve screenshots dir (tilde): {}", err),
        };
        let home = match dirs::home_dir() {
            Some(v) => v,
            None => panic!("Failed to resolve home directory"),
        };
        assert_eq!(result, home.join("Screenshots"));
    }

    #[test]
    fn test_generate_hyprland_binds() {
        let config = crate::config::Config::default();
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
        let config = crate::config::Config::default();
        let binds = config.generate_hyprland_binds_with_clipboard();

        assert!(binds.contains("bind = SUPER, Print, exec, hyprshot-rs -m window"));
        assert!(binds.contains("bind = SUPER SHIFT, Print, exec, hyprshot-rs -m region"));

        assert!(binds.contains("# Screenshot to clipboard (no file saved)"));
        assert!(
            binds.contains("bind = SUPER ALT, Print, exec, hyprshot-rs -m window --clipboard-only")
        );
        assert!(binds.contains(
            "bind = SUPER SHIFT ALT, Print, exec, hyprshot-rs -m region --clipboard-only"
        ));
        assert!(binds.contains(
            "bind = SUPER CTRL ALT, Print, exec, hyprshot-rs -m output --clipboard-only"
        ));
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use crate::windows_app;
    use crate::windows_capture;
    use crate::windows_capture::{CaptureRect, ScreenCapture};
    use crate::windows_i18n::WindowsLanguage;
    use crate::windows_icon;
    use crate::windows_ui;
    use std::path::PathBuf;
    use windows::Win32::Foundation::{LPARAM, POINT, RECT};
    #[cfg(feature = "windows-e2e")]
    use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
    use windows::Win32::UI::Input::KeyboardAndMouse::{MOD_CONTROL, MOD_SHIFT, VK_S};
    #[cfg(feature = "windows-e2e")]
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, WM_KEYDOWN};

    #[cfg(feature = "windows-e2e")]
    fn wait_for_window(class_name: &str, timeout: Duration) -> Option<HWND> {
        let start = Instant::now();
        let class = match class_name {
            "HyprshotOverlayWindow" => w!("HyprshotOverlayWindow"),
            "HyprshotWindowOverlay" => w!("HyprshotWindowOverlay"),
            "HyprshotMonitorOverlay" => w!("HyprshotMonitorOverlay"),
            _ => return None,
        };
        loop {
            let hwnd = unsafe { FindWindowW(class, None) };
            if hwnd.0 != 0 {
                return Some(hwnd);
            }
            if start.elapsed() > timeout {
                return None;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn temp_screenshots_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("hyprshot-tests").join(name);
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[cfg(feature = "windows-e2e")]
    fn integration_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn ui_logo_has_expected_dimensions() {
        let (dark_rgba, w, h) =
            windows_icon::ui_logo_rgba(true).expect("Expected dark UI logo rgba data");
        let (light_rgba, w2, h2) =
            windows_icon::ui_logo_rgba(false).expect("Expected light UI logo rgba data");

        assert!(w > 0 && h > 0);
        assert_eq!(w, w2);
        assert_eq!(h, h2);
        assert_eq!(dark_rgba.len(), (w * h * 4) as usize);
        assert_eq!(light_rgba.len(), (w2 * h2 * 4) as usize);
        assert_ne!(dark_rgba, light_rgba);
    }

    #[test]
    fn tray_icon_is_available() {
        assert!(windows_icon::tray_icon().is_some());
    }

    #[test]
    fn parse_hotkey_ctrl_shift_s() {
        let (mods, vk) =
            windows_app::parse_hotkey("CTRL SHIFT S").expect("Expected hotkey to parse");
        assert_ne!(mods.0 & MOD_CONTROL.0, 0);
        assert_ne!(mods.0 & MOD_SHIFT.0, 0);
        assert_eq!(vk, VK_S.0 as u32);
    }

    #[test]
    fn parse_hotkey_errors_on_missing_or_multiple_keys() {
        assert!(windows_app::parse_hotkey("SHIFT").is_err());
        assert!(windows_app::parse_hotkey("A B").is_err());
        assert!(windows_app::parse_hotkey("CTRL UNKNOWN").is_err());
    }

    #[test]
    fn dim_pixels_reduces_rgb() {
        let input = vec![100u8, 150, 200, 77, 10, 20, 30, 255];
        let output = windows_capture::dim_pixels(&input);
        assert_eq!(output.len(), input.len());
        assert_eq!(output[3], input[3]);
        assert_eq!(output[7], input[7]);
        assert!(output[0] < input[0]);
        assert!(output[1] < input[1]);
        assert!(output[2] < input[2]);
        assert!(output[4] < input[4]);
        assert!(output[5] < input[5]);
        assert!(output[6] < input[6]);
    }

    #[test]
    fn clamp_rect_to_bounds_limits_selection() {
        let rect = CaptureRect {
            left: -50,
            top: -20,
            right: 500,
            bottom: 80,
        };
        let clamped = windows_capture::clamp_rect_to_bounds(rect, 0, 0, 300, 100);
        assert_eq!(clamped.left, 0);
        assert_eq!(clamped.top, 0);
        assert_eq!(clamped.right, 300);
        assert_eq!(clamped.bottom, 80);
    }
    #[test]
    fn clamp_rect_to_bounds_handles_negative_monitor() {
        let rect = CaptureRect {
            left: -2500,
            top: -1200,
            right: -1000,
            bottom: -500,
        };
        let clamped = windows_capture::clamp_rect_to_bounds(rect, -1920, -1080, 1920, 1080);
        assert_eq!(clamped.left, -1920);
        assert_eq!(clamped.top, -1080);
        assert_eq!(clamped.right, -1000);
        assert_eq!(clamped.bottom, -500);
    }

    #[test]
    fn point_in_rect_inclusive_exclusive() {
        let rect = CaptureRect {
            left: 10,
            top: 20,
            right: 30,
            bottom: 40,
        };
        assert!(windows_capture::point_in_rect(POINT { x: 10, y: 20 }, rect));
        assert!(windows_capture::point_in_rect(POINT { x: 29, y: 39 }, rect));
        assert!(!windows_capture::point_in_rect(
            POINT { x: 30, y: 40 },
            rect
        ));
    }
    #[test]
    fn point_in_rect_with_negative_coords() {
        let rect = CaptureRect {
            left: -1920,
            top: -1080,
            right: -960,
            bottom: -540,
        };
        assert!(windows_capture::point_in_rect(
            POINT { x: -1000, y: -600 },
            rect
        ));
        assert!(!windows_capture::point_in_rect(
            POINT { x: -2000, y: -1200 },
            rect
        ));
    }

    #[test]
    fn union_rect_combines_bounds() {
        let a = RECT {
            left: 0,
            top: 0,
            right: 10,
            bottom: 10,
        };
        let b = RECT {
            left: 5,
            top: -5,
            right: 20,
            bottom: 12,
        };
        let out = windows_capture::union_rect(a, b);
        assert_eq!(out.left, 0);
        assert_eq!(out.top, -5);
        assert_eq!(out.right, 20);
        assert_eq!(out.bottom, 12);
    }

    #[test]
    fn inflate_rect_expands_bounds() {
        let rect = RECT {
            left: 10,
            top: 10,
            right: 20,
            bottom: 20,
        };
        let out = windows_capture::inflate_rect(rect, 5);
        assert_eq!(out.left, 5);
        assert_eq!(out.top, 5);
        assert_eq!(out.right, 25);
        assert_eq!(out.bottom, 25);
    }

    #[test]
    fn label_rect_at_clamps_to_edges() {
        let out = windows_capture::label_rect_at(POINT { x: 190, y: 190 }, 200, 200);
        assert_eq!(out.left, 100);
        assert_eq!(out.top, 176);
        assert_eq!(out.right, 200);
        assert_eq!(out.bottom, 200);
    }

    #[test]
    fn lparam_point_decodes_coords() {
        let x: i16 = 300;
        let y: i16 = 400;
        let packed = ((y as i32 as u32) << 16) | (x as i32 as u32 & 0xFFFF);
        let pt = windows_capture::lparam_point(LPARAM(packed as isize));
        assert_eq!(pt.x, x as i32);
        assert_eq!(pt.y, y as i32);
    }

    #[test]
    fn save_png_writes_file() {
        let dir = temp_screenshots_dir("save_png");
        let path = dir.join("test.png");
        let rgba = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];
        windows_capture::save_png(&path, &rgba, 2, 2).expect("Expected save_png to succeed");
        assert!(path.exists());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn crop_to_rgba_converts_bgra_and_crops() {
        let pixels = vec![1, 2, 3, 4, 10, 20, 30, 40, 5, 6, 7, 8, 50, 60, 70, 80];
        let capture = ScreenCapture {
            left: 0,
            top: 0,
            width: 2,
            height: 2,
            pixels,
        };
        let rect = CaptureRect {
            left: 1,
            top: 0,
            right: 2,
            bottom: 1,
        };
        let (rgba, w, h) =
            windows_capture::crop_to_rgba(&capture, &rect).expect("Expected crop to succeed");
        assert_eq!(w, 1);
        assert_eq!(h, 1);
        assert_eq!(rgba, vec![30, 20, 10, 40]);
    }
    #[test]
    fn crop_to_rgba_clamps_right_bottom_overflow() {
        let pixels = vec![1, 2, 3, 4, 10, 20, 30, 40, 5, 6, 7, 8, 50, 60, 70, 80];
        let capture = ScreenCapture {
            left: 0,
            top: 0,
            width: 2,
            height: 2,
            pixels,
        };
        let rect = CaptureRect {
            left: 1,
            top: 1,
            right: 5,
            bottom: 5,
        };
        let (rgba, w, h) =
            windows_capture::crop_to_rgba(&capture, &rect).expect("Expected crop to succeed");
        assert_eq!(w, 1);
        assert_eq!(h, 1);
        assert_eq!(rgba, vec![70, 60, 50, 80]);
    }

    #[test]
    fn crop_to_rgba_clamps_left_top_overflow() {
        let pixels = vec![1, 2, 3, 4, 10, 20, 30, 40, 5, 6, 7, 8, 50, 60, 70, 80];
        let capture = ScreenCapture {
            left: 0,
            top: 0,
            width: 2,
            height: 2,
            pixels,
        };
        let rect = CaptureRect {
            left: -3,
            top: -3,
            right: 1,
            bottom: 1,
        };
        let (rgba, w, h) =
            windows_capture::crop_to_rgba(&capture, &rect).expect("Expected crop to succeed");
        assert_eq!(w, 1);
        assert_eq!(h, 1);
        assert_eq!(rgba, vec![3, 2, 1, 4]);
    }

    #[test]
    fn crop_to_rgba_errors_on_zero_area() {
        let pixels = vec![1, 2, 3, 4, 10, 20, 30, 40, 5, 6, 7, 8, 50, 60, 70, 80];
        let capture = ScreenCapture {
            left: 0,
            top: 0,
            width: 2,
            height: 2,
            pixels,
        };
        let rect = CaptureRect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 1,
        };
        assert!(windows_capture::crop_to_rgba(&capture, &rect).is_err());
    }

    #[test]
    fn theme_toggle_tooltip_changes() {
        let dark = windows_ui::theme_toggle_tooltip(WindowsLanguage::En, true);
        let light = windows_ui::theme_toggle_tooltip(WindowsLanguage::En, false);
        assert!(!dark.is_empty());
        assert!(!light.is_empty());
        assert_ne!(dark, light);
    }
    #[cfg(feature = "windows-e2e")]
    #[test]
    fn integration_capture_region_overlay() {
        let _guard = integration_lock();
        let mut config = crate::config::Config::default();
        let dir = temp_screenshots_dir("region");
        config.paths.screenshots_dir = dir.to_string_lossy().to_string();
        unsafe {
            std::env::remove_var("HYPRSHOT_DIR");
        }
        let seen = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let seen_clone = seen.clone();

        let driver = std::thread::spawn(move || {
            if let Some(hwnd) = wait_for_window("HyprshotOverlayWindow", Duration::from_secs(5)) {
                seen_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(50));
                unsafe {
                    let _ = PostMessageW(hwnd, WM_KEYDOWN, WPARAM(VK_ESCAPE.0 as usize), LPARAM(0));
                }
            }
        });

        let _ = windows_capture::capture_region(&config);
        let _ = driver.join();
        assert!(seen.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[cfg(feature = "windows-e2e")]
    #[test]
    fn integration_capture_window_overlay() {
        let _guard = integration_lock();
        let mut config = crate::config::Config::default();
        let dir = temp_screenshots_dir("window");
        config.paths.screenshots_dir = dir.to_string_lossy().to_string();
        unsafe {
            std::env::remove_var("HYPRSHOT_DIR");
        }
        let seen = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let seen_clone = seen.clone();

        let driver = std::thread::spawn(move || {
            if let Some(hwnd) = wait_for_window("HyprshotWindowOverlay", Duration::from_secs(5)) {
                seen_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(50));
                unsafe {
                    let _ = PostMessageW(hwnd, WM_KEYDOWN, WPARAM(VK_ESCAPE.0 as usize), LPARAM(0));
                }
            }
        });

        let _ = windows_capture::capture_window(&config);
        let _ = driver.join();
        assert!(seen.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[cfg(feature = "windows-e2e")]
    #[test]
    fn integration_capture_output_overlay() {
        let _guard = integration_lock();
        let mut config = crate::config::Config::default();
        let dir = temp_screenshots_dir("output");
        config.paths.screenshots_dir = dir.to_string_lossy().to_string();
        unsafe {
            std::env::remove_var("HYPRSHOT_DIR");
        }
        let seen = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let seen_clone = seen.clone();

        let driver = std::thread::spawn(move || {
            if let Some(hwnd) = wait_for_window("HyprshotMonitorOverlay", Duration::from_secs(5)) {
                seen_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(50));
                unsafe {
                    let _ = PostMessageW(hwnd, WM_KEYDOWN, WPARAM(VK_ESCAPE.0 as usize), LPARAM(0));
                }
            }
        });

        let _ = windows_capture::capture_output(&config);
        let _ = driver.join();
        assert!(seen.load(std::sync::atomic::Ordering::SeqCst));
    }
}
