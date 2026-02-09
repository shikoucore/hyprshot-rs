#![allow(unsafe_op_in_unsafe_fn)]

use anyhow::Result;
use std::mem::size_of;
use std::sync::{Mutex, OnceLock};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, DEFAULT_GUI_FONT, DEFAULT_PITCH,
    DEFAULT_QUALITY, FF_SWISS, FW_NORMAL, GetStockObject, HFONT, OUT_DEFAULT_PRECIS,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RegCloseKey, RegDeleteValueW, RegOpenKeyExW,
    RegSetValueExW,
};
use windows::Win32::UI::Controls::{
    BST_CHECKED, BST_UNCHECKED, ICC_STANDARD_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx,
    SetWindowTheme,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, RegisterHotKey, UnregisterHotKey,
    VK_0, VK_1, VK_2, VK_3, VK_4, VK_5, VK_6, VK_7, VK_8, VK_9, VK_A, VK_B, VK_C, VK_D, VK_E,
    VK_ESCAPE, VK_F, VK_F1, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_F10, VK_F11,
    VK_F12, VK_G, VK_H, VK_I, VK_J, VK_K, VK_L, VK_M, VK_N, VK_O, VK_P, VK_Q, VK_R, VK_RETURN,
    VK_S, VK_SNAPSHOT, VK_SPACE, VK_T, VK_TAB, VK_U, VK_V, VK_W, VK_X, VK_Y, VK_Z,
};
use windows::Win32::UI::Shell::{
    FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, FileOpenDialog, IFileDialog, NIF_ICON, NIF_INFO,
    NIF_MESSAGE, NIF_TIP, NIIF_INFO, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
    SIGDN_FILESYSPATH, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, BM_GETCHECK, BM_SETCHECK, BS_AUTOCHECKBOX, BS_GROUPBOX, BS_PUSHBUTTON,
    CW_USEDEFAULT, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, ES_AUTOHSCROLL, GetCursorPos, GetDlgItem, GetMessageW, GetWindowTextW, HICON,
    HMENU, IDI_APPLICATION, LoadIconW, MF_STRING, MSG, PostQuitMessage, RegisterClassW, SW_HIDE,
    SendMessageW, SetForegroundWindow, SetWindowTextW, ShowWindow, TPM_RIGHTBUTTON, TrackPopupMenu,
    TranslateMessage, WINDOW_STYLE, WM_APP, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_HOTKEY,
    WM_LBUTTONDBLCLK, WM_RBUTTONUP, WM_SETFONT, WNDCLASSW, WS_CHILD, WS_EX_CLIENTEDGE,
    WS_EX_TOOLWINDOW, WS_POPUP, WS_TABSTOP, WS_VISIBLE,
};
use windows::core::{HSTRING, w};

use crate::windows_i18n::Strings;
use crate::windows_icon;

const WM_TRAYICON: u32 = WM_APP + 1;
const TRAY_ICON_ID: u32 = 1;
const MENU_SETTINGS: usize = 1001;
const MENU_EXIT: usize = 1002;

const ID_EDIT_SAVE_DIR: i32 = 2001;
const ID_BTN_BROWSE: i32 = 2002;
const ID_CHECK_NOTIF: i32 = 2003;
const ID_CHECK_FREEZE_ALL: i32 = 2004;
const ID_CHECK_AUTOSTART: i32 = 2005;
const ID_EDIT_HK_REGION: i32 = 2101;
const ID_EDIT_HK_WINDOW: i32 = 2102;
const ID_EDIT_HK_OUTPUT: i32 = 2103;
const ID_EDIT_HK_ACTIVE_OUTPUT: i32 = 2104;
const ID_BTN_SAVE: i32 = 2201;
const ID_BTN_CLOSE: i32 = 2202;

const HOTKEY_REGION: i32 = 1;
const HOTKEY_WINDOW: i32 = 2;
const HOTKEY_OUTPUT: i32 = 3;
const HOTKEY_ACTIVE_OUTPUT: i32 = 4;

static APP_STATE: OnceLock<Mutex<AppState>> = OnceLock::new();
static UI_FONT: OnceLock<HFONT> = OnceLock::new();

#[derive(Default)]
struct AppState {
    tray_hwnd: HWND,
    settings_hwnd: Option<HWND>,
}

pub fn run() -> Result<()> {
    ensure_default_config();

    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let icc = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_STANDARD_CLASSES,
        };
        let _ = InitCommonControlsEx(&icc);
        let hinstance: HINSTANCE = GetModuleHandleW(None)?.into();
        let tray_class = w!("HyprshotTrayWindow");
        let settings_class = w!("HyprshotSettingsWindow");
        let tray_wnd = WNDCLASSW {
            lpfnWndProc: Some(tray_wnd_proc),
            hInstance: hinstance,
            lpszClassName: tray_class,
            ..Default::default()
        };
        RegisterClassW(&tray_wnd);
        let settings_wnd = WNDCLASSW {
            lpfnWndProc: Some(settings_wnd_proc),
            hInstance: hinstance,
            lpszClassName: settings_class,
            ..Default::default()
        };
        RegisterClassW(&settings_wnd);
        let tray_hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            tray_class,
            w!("hyprshot-rs"),
            WS_POPUP,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            HWND(0),
            HMENU(0),
            hinstance,
            None,
        );
        if tray_hwnd.0 == 0 {
            return Ok(());
        }
        let state = AppState {
            tray_hwnd,
            settings_hwnd: None,
        };
        let _ = APP_STATE.set(Mutex::new(state));
        add_tray_icon(tray_hwnd)?;
        register_hotkeys(tray_hwnd, &load_config());
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(0), 0, 0).into() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}

fn ensure_default_config() {
    if !crate::config::Config::exists() {
        if let Ok(config) = crate::config::Config::load() {
            let _ = config.save();
        }
    }
}

unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => LRESULT(0),
        WM_TRAYICON => {
            let event = lparam.0 as u32;
            if event == WM_RBUTTONUP {
                show_tray_menu(hwnd);
                return LRESULT(0);
            }
            if event == WM_LBUTTONDBLCLK {
                show_settings_window();
                return LRESULT(0);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xffff) as usize;
            match id {
                MENU_SETTINGS => {
                    show_settings_window();
                }
                MENU_EXIT => {
                    let _ = DestroyWindow(hwnd);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            unregister_hotkeys(hwnd);
            let _ = remove_tray_icon(hwnd);
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_HOTKEY => {
            let id = wparam.0 as i32;
            handle_hotkey(id);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            init_settings_controls(hwnd);
            load_settings_into_controls(hwnd);
            LRESULT(0)
        }
        WM_CLOSE => {
            ShowWindow(hwnd, SW_HIDE);
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xffff) as i32;
            match id {
                ID_BTN_BROWSE => {
                    if let Some(path) = pick_folder(hwnd) {
                        set_edit_text(hwnd, ID_EDIT_SAVE_DIR, &path);
                    }
                }
                ID_BTN_SAVE => {
                    if let Err(err) = save_settings_from_controls(hwnd) {
                        show_error(hwnd, &format!("Failed to save settings: {err}"));
                    } else {
                        re_register_hotkeys(hwnd);
                        ShowWindow(hwnd, SW_HIDE);
                    }
                }
                ID_BTN_CLOSE => {
                    ShowWindow(hwnd, SW_HIDE);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            with_state(|state| {
                if state.settings_hwnd == Some(hwnd) {
                    state.settings_hwnd = None;
                }
            });
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn add_tray_icon(hwnd: HWND) -> Result<()> {
    let icon = load_app_icon();
    let mut nid = notify_icon_data(hwnd, icon);
    Shell_NotifyIconW(NIM_ADD, &mut nid);
    Ok(())
}

unsafe fn remove_tray_icon(hwnd: HWND) -> Result<()> {
    let icon = load_app_icon();
    let mut nid = notify_icon_data(hwnd, icon);
    Shell_NotifyIconW(NIM_DELETE, &mut nid);
    Ok(())
}

unsafe fn load_app_icon() -> HICON {
    windows_icon::tray_icon()
        .unwrap_or_else(|| LoadIconW(HINSTANCE(0), IDI_APPLICATION).unwrap_or(HICON(0)))
}

unsafe fn notify_icon_data(hwnd: HWND, icon: HICON) -> NOTIFYICONDATAW {
    let mut nid = NOTIFYICONDATAW::default();
    nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_ICON_ID;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAYICON;
    nid.hIcon = icon;
    copy_tip(&mut nid, "hyprshot-rs");
    nid
}

unsafe fn show_tray_menu(hwnd: HWND) {
    let menu = match CreatePopupMenu() {
        Ok(menu) => menu,
        Err(_) => return,
    };
    let strings = Strings::from_config(&load_config());
    let settings = HSTRING::from(strings.tray_settings());
    let exit = HSTRING::from(strings.tray_exit());
    let _ = AppendMenuW(menu, MF_STRING, MENU_SETTINGS, &settings);
    let _ = AppendMenuW(menu, MF_STRING, MENU_EXIT, &exit);
    let mut pt = POINT::default();
    if GetCursorPos(&mut pt).is_ok() {
        SetForegroundWindow(hwnd);
        TrackPopupMenu(menu, TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, None);
    }
    let _ = DestroyMenu(menu);
}

unsafe fn show_settings_window() {
    crate::windows_ui::show_settings_window();
}

fn with_state<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut AppState) -> R,
{
    APP_STATE.get().map(|state| {
        let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut guard)
    })
}

fn ui_font() -> HFONT {
    *UI_FONT.get_or_init(|| unsafe {
        let font = CreateFontW(
            -11,
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET.0 as u32,
            OUT_DEFAULT_PRECIS.0 as u32,
            CLIP_DEFAULT_PRECIS.0 as u32,
            DEFAULT_QUALITY.0 as u32,
            (DEFAULT_PITCH.0 | FF_SWISS.0) as u32,
            w!("Segoe UI"),
        );
        if font.0 != 0 {
            return font;
        }
        let stock = GetStockObject(DEFAULT_GUI_FONT);
        HFONT(stock.0)
    })
}

fn apply_control_style(hwnd: HWND, themed: bool) {
    unsafe {
        if hwnd.0 == 0 {
            return;
        }
        let font = ui_font();
        if font.0 != 0 {
            let _ = SendMessageW(hwnd, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));
        }
        if themed {
            let _ = SetWindowTheme(hwnd, w!("Explorer"), w!(""));
        }
    }
}

fn copy_tip(nid: &mut NOTIFYICONDATAW, text: &str) {
    let mut buf = [0u16; 128];
    for (i, ch) in text.encode_utf16().take(buf.len() - 1).enumerate() {
        buf[i] = ch;
    }
    nid.szTip = buf;
}

fn init_settings_controls(hwnd: HWND) {
    let strings = Strings::from_config(&load_config());
    unsafe {
        let pad = 20;
        let group_w = 680;
        let label_w = 170;
        let edit_w = 360;
        let row_h = 26;
        let gap = 16;
        let mut y = pad;
        create_groupbox(hwnd, pad, y, group_w, 90, strings.section_save());
        let inner_x = pad + 16;
        let inner_y = y + 30;
        create_label(
            hwnd,
            inner_x,
            inner_y,
            label_w,
            20,
            strings.label_screenshots_dir(),
        );
        create_edit(
            hwnd,
            ID_EDIT_SAVE_DIR,
            inner_x + label_w + 10,
            inner_y - 2,
            edit_w,
            row_h,
        );
        create_button(
            hwnd,
            ID_BTN_BROWSE,
            inner_x + label_w + 10 + edit_w + 10,
            inner_y - 3,
            110,
            row_h + 4,
            strings.button_browse(),
        );
        y += 90 + gap;
        create_groupbox(hwnd, pad, y, group_w, 180, strings.section_hotkeys());
        let inner_x = pad + 16;
        let mut row_y = y + 34;
        let edit_x = inner_x + label_w + 10;
        let hotkey_w = 220;
        create_label(hwnd, inner_x, row_y, label_w, 20, strings.label_region());
        create_edit(hwnd, ID_EDIT_HK_REGION, edit_x, row_y - 2, hotkey_w, row_h);
        row_y += 30;
        create_label(hwnd, inner_x, row_y, label_w, 20, strings.label_window());
        create_edit(hwnd, ID_EDIT_HK_WINDOW, edit_x, row_y - 2, hotkey_w, row_h);
        row_y += 30;
        create_label(hwnd, inner_x, row_y, label_w, 20, strings.label_monitor());
        create_edit(hwnd, ID_EDIT_HK_OUTPUT, edit_x, row_y - 2, hotkey_w, row_h);
        row_y += 30;
        create_label(
            hwnd,
            inner_x,
            row_y,
            label_w,
            20,
            strings.label_active_monitor(),
        );
        create_edit(
            hwnd,
            ID_EDIT_HK_ACTIVE_OUTPUT,
            edit_x,
            row_y - 2,
            hotkey_w,
            row_h,
        );
        y += 180 + gap;
        create_groupbox(hwnd, pad, y, group_w, 120, strings.section_behavior());
        let inner_x = pad + 16;
        let mut row_y = y + 32;
        create_checkbox(
            hwnd,
            ID_CHECK_NOTIF,
            inner_x,
            row_y,
            320,
            24,
            strings.checkbox_notifications(),
        );
        row_y += 28;
        create_checkbox(
            hwnd,
            ID_CHECK_FREEZE_ALL,
            inner_x,
            row_y,
            360,
            24,
            strings.checkbox_freeze_all(),
        );
        row_y += 28;
        create_checkbox(
            hwnd,
            ID_CHECK_AUTOSTART,
            inner_x,
            row_y,
            360,
            24,
            strings.checkbox_autostart(),
        );
        let btn_y = y + 120 + 18;
        create_button(
            hwnd,
            ID_BTN_SAVE,
            430,
            btn_y,
            120,
            32,
            strings.button_save(),
        );
        create_button(
            hwnd,
            ID_BTN_CLOSE,
            560,
            btn_y,
            120,
            32,
            strings.button_close(),
        );
    }
}

fn load_settings_into_controls(hwnd: HWND) {
    let config = load_config();
    set_edit_text(hwnd, ID_EDIT_SAVE_DIR, &config.paths.screenshots_dir);
    set_edit_text(hwnd, ID_EDIT_HK_REGION, &config.windows.hotkeys.region);
    set_edit_text(hwnd, ID_EDIT_HK_WINDOW, &config.windows.hotkeys.window);
    set_edit_text(hwnd, ID_EDIT_HK_OUTPUT, &config.windows.hotkeys.output);
    set_edit_text(
        hwnd,
        ID_EDIT_HK_ACTIVE_OUTPUT,
        &config.windows.hotkeys.active_output,
    );
    set_check(hwnd, ID_CHECK_NOTIF, config.windows.notifications.enabled);
    set_check(
        hwnd,
        ID_CHECK_FREEZE_ALL,
        config.windows.behavior.freeze_all_monitors,
    );
    set_check(hwnd, ID_CHECK_AUTOSTART, config.windows.behavior.autostart);
}

fn save_settings_from_controls(hwnd: HWND) -> Result<()> {
    let mut config = load_config();
    config.paths.screenshots_dir = get_edit_text(hwnd, ID_EDIT_SAVE_DIR);
    config.windows.hotkeys.region = get_edit_text(hwnd, ID_EDIT_HK_REGION);
    config.windows.hotkeys.window = get_edit_text(hwnd, ID_EDIT_HK_WINDOW);
    config.windows.hotkeys.output = get_edit_text(hwnd, ID_EDIT_HK_OUTPUT);
    config.windows.hotkeys.active_output = get_edit_text(hwnd, ID_EDIT_HK_ACTIVE_OUTPUT);
    config.windows.notifications.enabled = get_check(hwnd, ID_CHECK_NOTIF);
    config.windows.behavior.freeze_all_monitors = get_check(hwnd, ID_CHECK_FREEZE_ALL);
    config.windows.behavior.autostart = get_check(hwnd, ID_CHECK_AUTOSTART);
    config.save()?;
    set_autostart(config.windows.behavior.autostart)
        .map_err(|err| anyhow::anyhow!("Failed to configure autostart: {}", err))?;
    Ok(())
}

fn load_config() -> crate::config::Config {
    crate::config::Config::load().unwrap_or_else(|_| crate::config::Config::default())
}

fn set_edit_text(hwnd: HWND, id: i32, text: &str) {
    unsafe {
        if let Some(ctrl) = get_control(hwnd, id) {
            let value = HSTRING::from(text);
            let _ = SetWindowTextW(ctrl, &value);
        }
    }
}

fn get_edit_text(hwnd: HWND, id: i32) -> String {
    unsafe {
        let Some(ctrl) = get_control(hwnd, id) else {
            return String::new();
        };
        let mut buf = vec![0u16; 512];
        let len = GetWindowTextW(ctrl, &mut buf) as usize;
        String::from_utf16_lossy(&buf[..len])
    }
}

fn set_check(hwnd: HWND, id: i32, value: bool) {
    unsafe {
        if let Some(ctrl) = get_control(hwnd, id) {
            let state = if value { BST_CHECKED } else { BST_UNCHECKED };
            let _ = windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                ctrl,
                BM_SETCHECK,
                WPARAM(state.0 as usize),
                LPARAM(0),
            );
        }
    }
}

fn get_check(hwnd: HWND, id: i32) -> bool {
    unsafe {
        let Some(ctrl) = get_control(hwnd, id) else {
            return false;
        };
        let result = windows::Win32::UI::WindowsAndMessaging::SendMessageW(
            ctrl,
            BM_GETCHECK,
            WPARAM(0),
            LPARAM(0),
        );
        result.0 as u32 == BST_CHECKED.0
    }
}

unsafe fn get_control(hwnd: HWND, id: i32) -> Option<HWND> {
    let ctrl = GetDlgItem(hwnd, id);
    if ctrl.0 == 0 { None } else { Some(ctrl) }
}

unsafe fn create_label(hwnd: HWND, x: i32, y: i32, w: i32, h: i32, text: &str) -> HWND {
    let value = HSTRING::from(text);
    let ctrl = CreateWindowExW(
        Default::default(),
        w!("STATIC"),
        &value,
        WS_CHILD | WS_VISIBLE,
        x,
        y,
        w,
        h,
        hwnd,
        HMENU(0),
        HINSTANCE(0),
        None,
    );
    apply_control_style(ctrl, false);
    ctrl
}

unsafe fn create_edit(hwnd: HWND, id: i32, x: i32, y: i32, w: i32, h: i32) -> HWND {
    let style = WINDOW_STYLE((WS_CHILD | WS_VISIBLE | WS_TABSTOP).0 | ES_AUTOHSCROLL as u32);
    let ctrl = CreateWindowExW(
        WS_EX_CLIENTEDGE,
        w!("EDIT"),
        w!(""),
        style,
        x,
        y,
        w,
        h,
        hwnd,
        HMENU(id as isize),
        HINSTANCE(0),
        None,
    );
    apply_control_style(ctrl, true);
    ctrl
}

unsafe fn create_checkbox(hwnd: HWND, id: i32, x: i32, y: i32, w: i32, h: i32, text: &str) -> HWND {
    let value = HSTRING::from(text);
    let style = WINDOW_STYLE((WS_CHILD | WS_VISIBLE | WS_TABSTOP).0 | BS_AUTOCHECKBOX as u32);
    let ctrl = CreateWindowExW(
        Default::default(),
        w!("BUTTON"),
        &value,
        style,
        x,
        y,
        w,
        h,
        hwnd,
        HMENU(id as isize),
        HINSTANCE(0),
        None,
    );
    apply_control_style(ctrl, true);
    ctrl
}

unsafe fn create_button(hwnd: HWND, id: i32, x: i32, y: i32, w: i32, h: i32, text: &str) -> HWND {
    let value = HSTRING::from(text);
    let style = WINDOW_STYLE((WS_CHILD | WS_VISIBLE | WS_TABSTOP).0 | BS_PUSHBUTTON as u32);
    let ctrl = CreateWindowExW(
        Default::default(),
        w!("BUTTON"),
        &value,
        style,
        x,
        y,
        w,
        h,
        hwnd,
        HMENU(id as isize),
        HINSTANCE(0),
        None,
    );
    apply_control_style(ctrl, true);
    ctrl
}

unsafe fn create_groupbox(hwnd: HWND, x: i32, y: i32, w: i32, h: i32, text: &str) -> HWND {
    let value = HSTRING::from(text);
    let style = WINDOW_STYLE((WS_CHILD | WS_VISIBLE).0 | BS_GROUPBOX as u32);
    let ctrl = CreateWindowExW(
        Default::default(),
        w!("BUTTON"),
        &value,
        style,
        x,
        y,
        w,
        h,
        hwnd,
        HMENU(0),
        HINSTANCE(0),
        None,
    );
    apply_control_style(ctrl, false);
    ctrl
}

pub(crate) fn pick_folder(owner: HWND) -> Option<String> {
    unsafe {
        if CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_err() {
            return None;
        }
        let result = (|| {
            let dialog: IFileDialog =
                CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
            let options = dialog.GetOptions().unwrap_or_default();
            let _ = dialog.SetOptions(options | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM);
            if dialog.Show(owner).is_err() {
                return None;
            }
            let item = dialog.GetResult().ok()?;
            let path = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
            path.to_string().ok()
        })();
        CoUninitialize();
        result
    }
}

pub(crate) fn set_autostart(enabled: bool) -> Result<()> {
    unsafe {
        let mut key = Default::default();
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            0,
            KEY_SET_VALUE,
            &mut key,
        )
        .ok()?;
        if enabled {
            let exe = std::env::current_exe()?;
            let exe = exe.to_string_lossy();
            let data: Vec<u16> = exe.encode_utf16().chain(Some(0)).collect();
            let bytes = std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2);
            RegSetValueExW(key, w!("Hyprshot"), 0, REG_SZ, Some(bytes)).ok()?;
        } else {
            let _ = RegDeleteValueW(key, w!("Hyprshot"));
        }
        let _ = RegCloseKey(key);
        Ok(())
    }
}

pub(crate) fn apply_windows_config(config: &crate::config::Config) -> Result<()> {
    set_autostart(config.windows.behavior.autostart)
        .map_err(|err| anyhow::anyhow!("Failed to configure autostart: {}", err))?;
    refresh_hotkeys();
    Ok(())
}

fn show_error(hwnd: HWND, message: &str) {
    unsafe {
        let value = HSTRING::from(message);
        let _ = windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
            hwnd,
            &value,
            w!("hyprshot-rs"),
            windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
        );
    }
}

fn register_hotkeys(hwnd: HWND, config: &crate::config::Config) {
    let strings = Strings::from_config(config);
    let mut errors = Vec::new();

    register_one_hotkey(
        hwnd,
        HOTKEY_REGION,
        &config.windows.hotkeys.region,
        strings.label_region(),
        &mut errors,
    );
    register_one_hotkey(
        hwnd,
        HOTKEY_WINDOW,
        &config.windows.hotkeys.window,
        strings.label_window(),
        &mut errors,
    );
    register_one_hotkey(
        hwnd,
        HOTKEY_OUTPUT,
        &config.windows.hotkeys.output,
        strings.label_monitor(),
        &mut errors,
    );
    register_one_hotkey(
        hwnd,
        HOTKEY_ACTIVE_OUTPUT,
        &config.windows.hotkeys.active_output,
        strings.label_active_monitor(),
        &mut errors,
    );
    if !errors.is_empty() {
        let message = errors.join("\n");
        show_error(hwnd, &message);
    }
}

fn re_register_hotkeys(hwnd: HWND) {
    unregister_hotkeys(hwnd);
    register_hotkeys(hwnd, &load_config());
}

pub(crate) fn refresh_hotkeys() {
    let hwnd = with_state(|state| state.tray_hwnd).unwrap_or(HWND(0));
    if hwnd.0 != 0 {
        re_register_hotkeys(hwnd);
    }
}

fn unregister_hotkeys(hwnd: HWND) {
    unsafe {
        let _ = UnregisterHotKey(hwnd, HOTKEY_REGION);
        let _ = UnregisterHotKey(hwnd, HOTKEY_WINDOW);
        let _ = UnregisterHotKey(hwnd, HOTKEY_OUTPUT);
        let _ = UnregisterHotKey(hwnd, HOTKEY_ACTIVE_OUTPUT);
    }
}

fn register_one_hotkey(hwnd: HWND, id: i32, spec: &str, label: &str, errors: &mut Vec<String>) {
    match parse_hotkey(spec) {
        Ok((mods, vk)) => unsafe {
            if RegisterHotKey(hwnd, id, mods, vk).is_err() {
                errors.push(format!("Failed to register hotkey {}: {}", label, spec));
            }
        },
        Err(err) => errors.push(format!("Invalid hotkey {}: {} ({})", label, spec, err)),
    }
}

pub(crate) fn parse_hotkey(input: &str) -> Result<(HOT_KEY_MODIFIERS, u32)> {
    let normalized = input.replace('+', " ");
    let mut mods = HOT_KEY_MODIFIERS(0);
    let mut key: Option<u32> = None;
    for token in normalized.split_whitespace() {
        let t = token.trim().to_ascii_uppercase();
        if t.is_empty() {
            continue;
        }
        match t.as_str() {
            "CTRL" | "CONTROL" => mods |= MOD_CONTROL,
            "ALT" => mods |= MOD_ALT,
            "SHIFT" => mods |= MOD_SHIFT,
            "WIN" | "SUPER" | "META" => mods |= MOD_WIN,
            _ => {
                if key.is_some() {
                    return Err(anyhow::anyhow!("only one key can be specified"));
                }
                key = Some(parse_key(&t)?);
            }
        }
    }
    let Some(vk) = key else {
        return Err(anyhow::anyhow!("no key specified"));
    };
    Ok((mods, vk))
}

fn parse_key(token: &str) -> Result<u32> {
    let vk = match token {
        "A" => VK_A,
        "B" => VK_B,
        "C" => VK_C,
        "D" => VK_D,
        "E" => VK_E,
        "F" => VK_F,
        "G" => VK_G,
        "H" => VK_H,
        "I" => VK_I,
        "J" => VK_J,
        "K" => VK_K,
        "L" => VK_L,
        "M" => VK_M,
        "N" => VK_N,
        "O" => VK_O,
        "P" => VK_P,
        "Q" => VK_Q,
        "R" => VK_R,
        "S" => VK_S,
        "T" => VK_T,
        "U" => VK_U,
        "V" => VK_V,
        "W" => VK_W,
        "X" => VK_X,
        "Y" => VK_Y,
        "Z" => VK_Z,
        "0" => VK_0,
        "1" => VK_1,
        "2" => VK_2,
        "3" => VK_3,
        "4" => VK_4,
        "5" => VK_5,
        "6" => VK_6,
        "7" => VK_7,
        "8" => VK_8,
        "9" => VK_9,
        "F1" => VK_F1,
        "F2" => VK_F2,
        "F3" => VK_F3,
        "F4" => VK_F4,
        "F5" => VK_F5,
        "F6" => VK_F6,
        "F7" => VK_F7,
        "F8" => VK_F8,
        "F9" => VK_F9,
        "F10" => VK_F10,
        "F11" => VK_F11,
        "F12" => VK_F12,
        "ESC" | "ESCAPE" => VK_ESCAPE,
        "TAB" => VK_TAB,
        "SPACE" => VK_SPACE,
        "ENTER" | "RETURN" => VK_RETURN,
        "PRTSC" | "PRINTSCREEN" | "PRNTSCR" => VK_SNAPSHOT,
        _ => return Err(anyhow::anyhow!("unknown key: {}", token)),
    };
    Ok(vk.0 as u32)
}

fn handle_hotkey(id: i32) {
    match id {
        HOTKEY_REGION => {
            let config = load_config();
            let strings = Strings::from_config(&config);
            if let Err(err) = crate::windows_capture::capture_region(&config) {
                show_error_message(&format!("Failed to take screenshot: {err}"));
            } else {
                show_capture_notification(
                    &config,
                    strings.screenshot_region_title(),
                    strings.screenshot_saved(),
                );
            }
        }
        HOTKEY_WINDOW => {
            let config = load_config();
            let strings = Strings::from_config(&config);
            if let Err(err) = crate::windows_capture::capture_window(&config) {
                show_error_message(&format!("Failed to take screenshot: {err}"));
            } else {
                show_capture_notification(
                    &config,
                    strings.screenshot_window_title(),
                    strings.screenshot_saved(),
                );
            }
        }
        HOTKEY_OUTPUT => {
            let config = load_config();
            let strings = Strings::from_config(&config);
            if let Err(err) = crate::windows_capture::capture_output(&config) {
                show_error_message(&format!("Failed to take screenshot: {err}"));
            } else {
                show_capture_notification(
                    &config,
                    strings.screenshot_monitor_title(),
                    strings.screenshot_saved(),
                );
            }
        }
        HOTKEY_ACTIVE_OUTPUT => {
            let config = load_config();
            let strings = Strings::from_config(&config);
            if let Err(err) = crate::windows_capture::capture_active_output(&config) {
                show_error_message(&format!("Failed to take screenshot: {err}"));
            } else {
                show_capture_notification(
                    &config,
                    strings.screenshot_active_monitor_title(),
                    strings.screenshot_saved(),
                );
            }
        }
        _ => {}
    }
}

fn show_capture_notification(config: &crate::config::Config, title: &str, message: &str) {
    if !config.windows.notifications.enabled {
        return;
    }
    show_tray_notification(title, message);
}

fn show_tray_notification(title: &str, message: &str) {
    unsafe {
        let tray_hwnd = with_state(|state| state.tray_hwnd).unwrap_or(HWND(0));
        if tray_hwnd.0 == 0 {
            return;
        }

        let icon = load_app_icon();
        let mut nid = notify_icon_data(tray_hwnd, icon);
        nid.uFlags |= NIF_INFO;
        copy_wide(&mut nid.szInfo, message);
        copy_wide(&mut nid.szInfoTitle, title);
        nid.dwInfoFlags = NIIF_INFO;
        nid.Anonymous.uTimeout = 5000;
        Shell_NotifyIconW(NIM_MODIFY, &mut nid);
    }
}

fn copy_wide(buf: &mut [u16], text: &str) {
    for (i, ch) in text
        .encode_utf16()
        .take(buf.len().saturating_sub(1))
        .enumerate()
    {
        buf[i] = ch;
    }
}

fn show_error_message(message: &str) {
    unsafe {
        let value = HSTRING::from(message);
        let _ = windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
            HWND(0),
            &value,
            w!("hyprshot-rs"),
            windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
        );
    }
}
