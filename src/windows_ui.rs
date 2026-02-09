use std::process::Command;

use anyhow::Result;
use eframe::{App, Frame, NativeOptions, egui};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MessageBoxW};
use windows::core::HSTRING;

use crate::config;
use crate::windows_i18n::{Strings, WindowsLanguage};
use crate::windows_icon;

const UI_ROW_H: f32 = 22.0;
const UI_LABEL_W: f32 = 150.0;
const UI_HOTKEY_LABEL_W: f32 = 120.0;
const UI_HOTKEY_FIELD_W: f32 = 180.0;
const UI_BROWSE_W: f32 = 84.0;
const UI_BUTTON_W: f32 = 96.0;
const UI_COMBO_W: f32 = 140.0;
const UI_BLOCK_TALL_H: f32 = 170.0;
const UI_GROUP_MARGIN: f32 = 8.0;
const UI_COL_GAP: f32 = 12.0;
const UI_ROW_GAP: f32 = 8.0;
pub fn show_settings_window() {
    let exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(err) => {
            show_error(&format!("Failed to determine application path: {err}"));
            return;
        }
    };
    if let Err(err) = Command::new(exe).arg("--settings-ui").spawn() {
        show_error(&format!("Failed to open settings: {err}"));
    }
}

pub fn run_settings() -> Result<()> {
    let config = config::Config::load().unwrap_or_else(|_| config::Config::default());
    let strings = Strings::from_config(&config);
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([800.0, 520.0])
        .with_resizable(false);
    if let Some((rgba, width, height)) = windows_icon::ui_logo_rgba(true) {
        viewport = viewport.with_icon(egui::IconData {
            rgba,
            width,
            height,
        });
    }
    let options = NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        strings.settings_title(),
        options,
        Box::new(|_cc| Ok(Box::new(SettingsApp::load()))),
    )
    .map_err(|err| anyhow::anyhow!("Failed to launch settings window: {}", err))?;
    Ok(())
}

fn show_error(message: &str) {
    unsafe {
        let text = HSTRING::from(message);
        let _ = MessageBoxW(HWND(0), &text, &HSTRING::from("hyprshot-rs"), MB_ICONERROR);
    }
}

struct SettingsApp {
    screenshots_dir: String,
    hotkey_region: String,
    hotkey_window: String,
    hotkey_output: String,
    hotkey_active_output: String,
    notifications: bool,
    freeze_all: bool,
    autostart: bool,
    language: WindowsLanguage,
    last_font_language: Option<WindowsLanguage>,
    status: Option<String>,
    dark_mode: bool,
    logo_light: Option<egui::TextureHandle>,
    logo_dark: Option<egui::TextureHandle>,
}

impl SettingsApp {
    fn load() -> Self {
        let config = config::Config::load().unwrap_or_else(|_| config::Config::default());
        Self {
            screenshots_dir: config.paths.screenshots_dir,
            hotkey_region: config.windows.hotkeys.region,
            hotkey_window: config.windows.hotkeys.window,
            hotkey_output: config.windows.hotkeys.output,
            hotkey_active_output: config.windows.hotkeys.active_output,
            notifications: config.windows.notifications.enabled,
            freeze_all: config.windows.behavior.freeze_all_monitors,
            autostart: config.windows.behavior.autostart,
            language: WindowsLanguage::from_code(&config.windows.behavior.language),
            last_font_language: None,
            status: None,
            dark_mode: true,
            logo_light: None,
            logo_dark: None,
        }
    }

    fn ensure_logo(&mut self, ctx: &egui::Context) {
        if self.logo_light.is_none() {
            if let Some((rgba, width, height)) = windows_icon::ui_logo_rgba(false) {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [width as usize, height as usize],
                    &rgba,
                );
                let texture = ctx.load_texture("hyprshot_logo_light", image, Default::default());
                self.logo_light = Some(texture);
            }
        }
        if self.logo_dark.is_none() {
            if let Some((rgba, width, height)) = windows_icon::ui_logo_rgba(true) {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [width as usize, height as usize],
                    &rgba,
                );
                let texture = ctx.load_texture("hyprshot_logo_dark", image, Default::default());
                self.logo_dark = Some(texture);
            }
        }
    }

    fn current_logo(&self) -> Option<&egui::TextureHandle> {
        if self.dark_mode {
            self.logo_dark.as_ref().or(self.logo_light.as_ref())
        } else {
            self.logo_light.as_ref().or(self.logo_dark.as_ref())
        }
    }

    fn apply_to_config(&self, config: &mut config::Config) {
        config.paths.screenshots_dir = self.screenshots_dir.clone();
        config.windows.hotkeys.region = self.hotkey_region.clone();
        config.windows.hotkeys.window = self.hotkey_window.clone();
        config.windows.hotkeys.output = self.hotkey_output.clone();
        config.windows.hotkeys.active_output = self.hotkey_active_output.clone();
        config.windows.notifications.enabled = self.notifications;
        config.windows.behavior.freeze_all_monitors = self.freeze_all;
        config.windows.behavior.autostart = self.autostart;
        config.windows.behavior.language = self.language.code().to_string();
    }

    fn save(&mut self) -> Result<()> {
        let mut cfg = config::Config::load().unwrap_or_else(|_| config::Config::default());
        self.apply_to_config(&mut cfg);
        cfg.save()?;
        crate::windows_app::apply_windows_config(&cfg)?;
        Ok(())
    }
}

impl App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        if self.last_font_language != Some(self.language) {
            configure_fonts(ctx, self.language);
            self.last_font_language = Some(self.language);
        }
        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }
        let strings = Strings::new(self.language);
        self.ensure_logo(ctx);
        egui::TopBottomPanel::top("top")
            .exact_height(50.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.columns(2, |columns| {
                    let (left, right) = columns.split_at_mut(1);
                    let left = &mut left[0];
                    let right = &mut right[0];

                    left.horizontal(|ui| {
                        if let Some(logo) = self.current_logo() {
                            ui.add(
                                egui::Image::new(logo).fit_to_exact_size(egui::vec2(28.0, 28.0)),
                            );
                        }
                        ui.label(
                            egui::RichText::new(strings.settings_label())
                                .strong()
                                .size(18.0),
                        );
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(strings.app_name()).size(13.0));
                    });

                    right.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if draw_theme_toggle(ui, self.dark_mode, self.language) {
                            self.dark_mode = !self.dark_mode;
                        }
                    });
                });
            });
        egui::TopBottomPanel::bottom("bottom")
            .exact_height(56.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                let avail = ui.available_width();
                let button_w = UI_BUTTON_W;
                let gap = 12.0;
                let row_w = button_w * 2.0 + gap;
                let left_pad = ((avail - row_w) / 2.0).max(0.0);
                ui.horizontal(|ui| {
                    ui.add_space(left_pad);
                    if ui
                        .add_sized(
                            [button_w, UI_ROW_H],
                            egui::Button::new(strings.button_save()),
                        )
                        .clicked()
                    {
                        match self.save() {
                            Ok(_) => self.status = Some(strings.status_saved().to_string()),
                            Err(err) => self.status = Some(format!("Error: {}", err)),
                        }
                    }
                    ui.add_space(gap);
                    if ui
                        .add_sized(
                            [button_w, UI_ROW_H],
                            egui::Button::new(strings.button_close()),
                        )
                        .clicked()
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                if let Some(status) = &self.status {
                    let color = if self.dark_mode {
                        egui::Color32::from_rgb(120, 200, 140)
                    } else {
                        egui::Color32::from_rgb(40, 120, 70)
                    };
                    ui.add_space(4.0);
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new(status).color(color));
                    });
                }
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(10.0);
            ui.spacing_mut().item_spacing = egui::vec2(UI_ROW_GAP, UI_ROW_GAP);
            let total_w = (ui.available_width() - 6.0).max(0.0);
            let right_w = 240.0;
            let left_w = (total_w - right_w - UI_COL_GAP).max(420.0);
            ui.horizontal(|ui| {
                ui.set_min_width(total_w);
                ui.vertical(|ui| {
                    ui.set_min_width(left_w);
                    ui.set_max_width(left_w);
                    draw_save_section(ui, self, &strings);
                });
                ui.add_space(UI_COL_GAP);
                ui.vertical(|ui| {
                    ui.set_min_width(right_w);
                    ui.set_max_width(right_w);
                    draw_language_section(ui, self, &strings);
                });
            });
            ui.add_space(UI_ROW_GAP);
            ui.horizontal(|ui| {
                ui.set_min_width(total_w);
                ui.vertical(|ui| {
                    ui.set_min_width(left_w);
                    ui.set_max_width(left_w);
                    draw_hotkeys_section(ui, self, &strings);
                });
                ui.add_space(UI_COL_GAP);
                ui.vertical(|ui| {
                    ui.set_min_width(right_w);
                    ui.set_max_width(right_w);
                    draw_behavior_section(ui, self, &strings);
                });
            });
        });
    }
}

fn draw_save_section(ui: &mut egui::Ui, app: &mut SettingsApp, strings: &Strings) {
    group_frame(ui).show(ui, |ui| {
        ui.set_min_height(top_block_height(ui));
        ui.set_min_width(ui.available_width());
        ui.label(egui::RichText::new(strings.section_save()).strong());
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let label_w = UI_LABEL_W;
            let browse_w = UI_BROWSE_W;
            let spacing = ui.spacing().item_spacing.x * 2.0;
            let field_w = (ui.available_width() - label_w - browse_w - spacing).max(140.0);
            ui.add_sized(
                [label_w, UI_ROW_H],
                egui::Label::new(strings.label_screenshots_dir()),
            );
            ui.add_sized(
                [field_w, UI_ROW_H],
                egui::TextEdit::singleline(&mut app.screenshots_dir),
            );
            if ui
                .add_sized(
                    [browse_w, UI_ROW_H],
                    egui::Button::new(strings.button_browse()),
                )
                .clicked()
            {
                if let Some(path) = crate::windows_app::pick_folder(HWND(0)) {
                    app.screenshots_dir = path;
                }
            }
        });
    });
}

fn draw_behavior_section(ui: &mut egui::Ui, app: &mut SettingsApp, strings: &Strings) {
    group_frame(ui).show(ui, |ui| {
        ui.set_min_height(UI_BLOCK_TALL_H);
        ui.set_min_width(ui.available_width());
        ui.label(egui::RichText::new(strings.section_behavior()).strong());
        ui.add_space(8.0);
        ui.checkbox(&mut app.notifications, strings.checkbox_notifications());
        ui.checkbox(&mut app.freeze_all, strings.checkbox_freeze_all());
        ui.checkbox(&mut app.autostart, strings.checkbox_autostart());
    });
}

fn draw_language_section(ui: &mut egui::Ui, app: &mut SettingsApp, strings: &Strings) {
    group_frame(ui).show(ui, |ui| {
        ui.set_min_height(top_block_height(ui));
        ui.set_min_width(ui.available_width());
        ui.label(egui::RichText::new(strings.section_language()).strong());
        ui.add_space(8.0);
        egui::ComboBox::from_id_source("language_select")
            .selected_text(app.language.display_name())
            .width(UI_COMBO_W)
            .show_ui(ui, |ui| {
                for lang in WindowsLanguage::ALL {
                    ui.selectable_value(&mut app.language, lang, lang.display_name());
                }
            });
    });
}

fn draw_hotkeys_section(ui: &mut egui::Ui, app: &mut SettingsApp, strings: &Strings) {
    group_frame(ui).show(ui, |ui| {
        ui.set_min_height(UI_BLOCK_TALL_H);
        ui.set_min_width(ui.available_width());
        ui.label(egui::RichText::new(strings.section_hotkeys()).strong());
        ui.add_space(8.0);
        let hotkey_label_w = UI_HOTKEY_LABEL_W;
        let available = ui.available_width();
        let spacing = ui.spacing().item_spacing.x;
        let mut field_w = UI_HOTKEY_FIELD_W;
        let max_field = (available - hotkey_label_w - spacing).max(120.0);
        if field_w > max_field {
            field_w = max_field;
        }
        egui::Grid::new("hotkeys_grid")
            .num_columns(2)
            .spacing([12.0, 10.0])
            .show(ui, |ui| {
                ui.add_sized(
                    [hotkey_label_w, UI_ROW_H],
                    egui::Label::new(strings.label_region()),
                );
                ui.add_sized(
                    [field_w, UI_ROW_H],
                    egui::TextEdit::singleline(&mut app.hotkey_region),
                );
                ui.end_row();

                ui.add_sized(
                    [hotkey_label_w, UI_ROW_H],
                    egui::Label::new(strings.label_window()),
                );
                ui.add_sized(
                    [field_w, UI_ROW_H],
                    egui::TextEdit::singleline(&mut app.hotkey_window),
                );
                ui.end_row();

                ui.add_sized(
                    [hotkey_label_w, UI_ROW_H],
                    egui::Label::new(strings.label_monitor()),
                );
                ui.add_sized(
                    [field_w, UI_ROW_H],
                    egui::TextEdit::singleline(&mut app.hotkey_output),
                );
                ui.end_row();

                ui.add_sized(
                    [hotkey_label_w, UI_ROW_H],
                    egui::Label::new(strings.label_active_monitor()),
                );
                ui.add_sized(
                    [field_w, UI_ROW_H],
                    egui::TextEdit::singleline(&mut app.hotkey_active_output),
                );
                ui.end_row();
            });
    });
}

fn configure_fonts(ctx: &egui::Context, lang: WindowsLanguage) {
    if lang == WindowsLanguage::En {
        ctx.set_fonts(egui::FontDefinitions::default());
        return;
    }
    let Some(jp_font) = load_japanese_font() else {
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("jp".to_string(), jp_font);
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        family.push("jp".to_string());
    }
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
        family.push("jp".to_string());
    }
    ctx.set_fonts(fonts);
}

fn group_frame(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::group(ui.style()).inner_margin(egui::Margin::same(UI_GROUP_MARGIN))
}

fn top_block_height(ui: &egui::Ui) -> f32 {
    let label_h = ui.text_style_height(&egui::TextStyle::Body);
    UI_GROUP_MARGIN + label_h + 8.0 + UI_ROW_H + UI_GROUP_MARGIN
}

fn load_japanese_font() -> Option<egui::FontData> {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
    let fonts_dir = std::path::Path::new(&windir).join("Fonts");
    let candidates = [
        "YuGothR.ttc",
        "YuGothM.ttc",
        "YuGothB.ttc",
        "Meiryo.ttc",
        "MeiryoUI.ttf",
        "Meiryo.ttf",
    ];
    for name in candidates {
        let path = fonts_dir.join(name);
        if let Ok(bytes) = std::fs::read(&path) {
            return Some(egui::FontData::from_owned(bytes));
        }
    }
    None
}

fn draw_theme_toggle(ui: &mut egui::Ui, dark_mode: bool, lang: WindowsLanguage) -> bool {
    let size = egui::vec2(32.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let painter = ui.painter_at(rect);
    let visuals = ui.style().interact(&response);
    let bg = ui.visuals().window_fill;
    painter.rect(rect, 6.0, visuals.bg_fill, visuals.bg_stroke);
    let center = rect.center();
    let radius = 7.5;
    if dark_mode {
        let sun = egui::Color32::from_rgb(255, 205, 90);
        let ray = egui::Stroke::new(1.6, sun);
        painter.circle_filled(center, radius, sun);
        for i in 0..8 {
            let angle = (i as f32) * std::f32::consts::TAU / 8.0;
            let dir = egui::vec2(angle.cos(), angle.sin());
            let p1 = center + dir * (radius + 2.5);
            let p2 = center + dir * (radius + 6.5);
            painter.line_segment([p1, p2], ray);
        }
    } else {
        let moon = egui::Color32::from_rgb(210, 220, 245);
        let stroke = egui::Stroke::new(1.2, moon);
        painter.circle_filled(center, radius, moon);
        let cut = center + egui::vec2(3.0, -2.0);
        painter.circle_filled(cut, radius * 0.9, bg);
        painter.circle_stroke(center, radius, stroke);
    }
    let tooltip = theme_toggle_tooltip(lang, dark_mode);
    response.on_hover_text(tooltip).clicked()
}

pub(crate) fn theme_toggle_tooltip(lang: WindowsLanguage, dark_mode: bool) -> &'static str {
    let strings = Strings::new(lang);
    if dark_mode {
        strings.theme_light()
    } else {
        strings.theme_dark()
    }
}
