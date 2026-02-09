#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsLanguage {
    En,
    Ja,
}

impl WindowsLanguage {
    pub const ALL: [WindowsLanguage; 2] = [WindowsLanguage::En, WindowsLanguage::Ja];

    pub fn from_code(code: &str) -> Self {
        match code.trim().to_ascii_lowercase().as_str() {
            "ja" | "jp" | "jpn" => WindowsLanguage::Ja,
            _ => WindowsLanguage::En,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            WindowsLanguage::En => "en",
            WindowsLanguage::Ja => "ja",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            WindowsLanguage::En => "English",
            WindowsLanguage::Ja => "日本語",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Strings {
    lang: WindowsLanguage,
}

impl Strings {
    pub fn new(lang: WindowsLanguage) -> Self {
        Self { lang }
    }

    pub fn from_config(config: &crate::config::Config) -> Self {
        let lang = WindowsLanguage::from_code(&config.windows.behavior.language);
        Self::new(lang)
    }

    pub fn app_name(self) -> &'static str {
        "hyprshot-rs"
    }

    pub fn settings_title(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "hyprshot-rs Settings",
            WindowsLanguage::Ja => "hyprshot-rs 設定",
        }
    }

    pub fn settings_label(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Settings",
            WindowsLanguage::Ja => "設定",
        }
    }

    pub fn section_save(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Save",
            WindowsLanguage::Ja => "保存",
        }
    }

    pub fn section_behavior(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Behavior",
            WindowsLanguage::Ja => "動作",
        }
    }

    pub fn section_language(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Language",
            WindowsLanguage::Ja => "言語",
        }
    }

    pub fn section_hotkeys(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Hotkeys",
            WindowsLanguage::Ja => "ホットキー",
        }
    }

    pub fn label_screenshots_dir(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Screenshots folder",
            WindowsLanguage::Ja => "保存先",
        }
    }

    pub fn button_browse(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Browse...",
            WindowsLanguage::Ja => "参照...",
        }
    }

    pub fn label_language(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Language",
            WindowsLanguage::Ja => "言語",
        }
    }

    pub fn checkbox_notifications(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Notifications enabled",
            WindowsLanguage::Ja => "通知を有効化",
        }
    }

    pub fn checkbox_freeze_all(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Freeze all monitors",
            WindowsLanguage::Ja => "全モニターをフリーズ",
        }
    }

    pub fn checkbox_autostart(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Start with Windows",
            WindowsLanguage::Ja => "Windows 起動時に開始",
        }
    }

    pub fn label_region(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Region",
            WindowsLanguage::Ja => "範囲",
        }
    }

    pub fn label_window(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Window",
            WindowsLanguage::Ja => "ウィンドウ",
        }
    }

    pub fn label_monitor(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Monitor",
            WindowsLanguage::Ja => "モニター",
        }
    }

    pub fn label_active_monitor(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Active monitor",
            WindowsLanguage::Ja => "アクティブ",
        }
    }

    pub fn button_save(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Save",
            WindowsLanguage::Ja => "保存",
        }
    }

    pub fn button_close(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Close",
            WindowsLanguage::Ja => "閉じる",
        }
    }

    pub fn status_saved(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Settings saved",
            WindowsLanguage::Ja => "設定を保存しました",
        }
    }

    pub fn theme_light(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Light theme",
            WindowsLanguage::Ja => "ライトテーマ",
        }
    }

    pub fn theme_dark(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Dark theme",
            WindowsLanguage::Ja => "ダークテーマ",
        }
    }

    pub fn tray_settings(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Settings",
            WindowsLanguage::Ja => "設定",
        }
    }

    pub fn tray_exit(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Exit",
            WindowsLanguage::Ja => "終了",
        }
    }

    pub fn screenshot_region_title(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Region screenshot",
            WindowsLanguage::Ja => "範囲のスクリーンショット",
        }
    }

    pub fn screenshot_window_title(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Window screenshot",
            WindowsLanguage::Ja => "ウィンドウのスクリーンショット",
        }
    }

    pub fn screenshot_monitor_title(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Monitor screenshot",
            WindowsLanguage::Ja => "モニターのスクリーンショット",
        }
    }

    pub fn screenshot_active_monitor_title(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Active monitor screenshot",
            WindowsLanguage::Ja => "アクティブモニターのスクリーンショット",
        }
    }

    pub fn screenshot_saved(self) -> &'static str {
        match self.lang {
            WindowsLanguage::En => "Screenshot saved",
            WindowsLanguage::Ja => "スクリーンショットを保存しました",
        }
    }
}
