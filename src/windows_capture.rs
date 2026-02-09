#![allow(unsafe_op_in_unsafe_fn)]

use crate::config;
use anyhow::{Context, Result};
use arboard::{Clipboard, ImageData};
use std::borrow::Cow;
use std::mem::size_of;
use std::path::Path;
use std::ptr::null_mut;
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::SizeInt32;
use windows::Win32::Foundation::{
    BOOL, COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAP_READ,
    D3D11_MAPPED_SUBRESOURCE, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmFlush, DwmGetWindowAttribute};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BeginPaint, BitBlt, CAPTUREBLT, CreateCompatibleDC,
    CreateDIBSection, CreatePen, CreateRectRgn, DIB_RGB_COLORS, DeleteDC, DeleteObject, EndPaint,
    GetDC, GetMonitorInfoW, GetStockObject, HBITMAP, HDC, HGDIOBJ, HMONITOR, HRGN, InvalidateRect,
    MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, NULL_BRUSH, PAINTSTRUCT, PS_SOLID,
    ReleaseDC, SRCCOPY, SelectClipRgn, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
    TextOutW,
};
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, ReleaseCapture, SetCapture, VK_ESCAPE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GA_ROOT, GW_HWNDNEXT, GWL_EXSTYLE, GetAncestor, GetCursorPos, GetSystemMetrics, GetWindow,
    GetWindowLongW, GetWindowRect, HMENU, HWND_NOTOPMOST, HWND_TOPMOST, IDC_CROSS, IsIconic,
    IsWindowVisible, LoadCursorW, MSG, PM_REMOVE, PeekMessageW, RegisterClassW, SM_CXVIRTUALSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_HIDE, SW_SHOW, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SetCursor, SetForegroundWindow, SetWindowPos, ShowWindow,
    TranslateMessage, WM_CREATE, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETCURSOR, WNDCLASSW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, WS_VISIBLE,
    WindowFromPoint,
};
use windows::core::Interface;
use windows::core::w;

pub fn capture_region(config: &config::Config) -> Result<()> {
    let target = capture_area(config.windows.behavior.freeze_all_monitors)?;
    let capture = capture_screen(&target)?;
    let selection = match run_region_overlay(capture.clone())? {
        Some(rect) => rect,
        None => return Ok(()),
    };
    let save_dir = config::get_screenshots_dir(None, config, false)?;
    let save_dir = config::ensure_directory(&save_dir.to_string_lossy())?;
    let filename = default_filename();
    let save_path = save_dir.join(filename);
    let (rgba, width, height) = crop_to_rgba(&capture, &selection)?;
    save_png(&save_path, &rgba, width, height)?;
    copy_to_clipboard(&rgba, width, height).ok();
    Ok(())
}

fn is_window_topmost(hwnd: HWND) -> bool {
    unsafe { (GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 as u32) != 0 }
}

fn set_window_topmost(hwnd: HWND, topmost: bool) {
    unsafe {
        let insert_after = if topmost {
            HWND_TOPMOST
        } else {
            HWND_NOTOPMOST
        };
        let _ = SetWindowPos(
            hwnd,
            insert_after,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

pub fn capture_window(config: &config::Config) -> Result<()> {
    let target = capture_area(config.windows.behavior.freeze_all_monitors)?;
    let background = capture_screen(&target)?;
    let selection = match run_window_overlay(background.clone())? {
        Some(sel) => sel,
        None => return Ok(()),
    };
    let was_topmost = is_window_topmost(selection.hwnd);
    set_window_topmost(selection.hwnd, true);
    unsafe {
        let _ = DwmFlush();
    }
    std::thread::sleep(std::time::Duration::from_millis(40));
    let post_raise_background = capture_screen(&target).unwrap_or_else(|_| background.clone());
    let (capture, used_background) = if WINDOW_CAPTURE_USE_WGC {
        match capture_window_wgc(selection.hwnd, selection.rect) {
            Ok(c) => (c, false),
            Err(_) => (post_raise_background.clone(), true),
        }
    } else {
        match capture_window_print(selection.hwnd) {
            Ok(c) => (c, false),
            Err(_) => (post_raise_background.clone(), true),
        }
    };
    if !was_topmost {
        set_window_topmost(selection.hwnd, false);
    }
    let save_dir = config::get_screenshots_dir(None, config, false)?;
    let save_dir = config::ensure_directory(&save_dir.to_string_lossy())?;
    let filename = default_filename();
    let save_path = save_dir.join(filename);
    let (rgba, width, height) = if capture.left == selection.rect.left
        && capture.top == selection.rect.top
        && capture.width == selection.rect.width()
        && capture.height == selection.rect.height()
    {
        bgra_to_rgba_full(&capture)
    } else if used_background {
        crop_to_rgba(&capture, &selection.rect)?
    } else {
        bgra_to_rgba_full(&capture)
    };
    save_png(&save_path, &rgba, width, height)?;
    copy_to_clipboard(&rgba, width, height).ok();
    Ok(())
}

pub fn capture_output(config: &config::Config) -> Result<()> {
    let target = capture_area(true)?;
    let background = capture_screen_gdi(&target.rect)?;
    let selection = match run_monitor_overlay(background.clone())? {
        Some(sel) => sel,
        None => return Ok(()),
    };
    let (capture, used_background) = if MONITOR_CAPTURE_USE_WGC {
        if let Some(monitor) = selection.monitor {
            match capture_screen_wgc(monitor, &selection.rect) {
                Ok(c) => (c, false),
                Err(_) => (background.clone(), true),
            }
        } else {
            (background.clone(), true)
        }
    } else {
        (background.clone(), true)
    };
    let save_dir = config::get_screenshots_dir(None, config, false)?;
    let save_dir = config::ensure_directory(&save_dir.to_string_lossy())?;
    let filename = default_filename();
    let save_path = save_dir.join(filename);
    let (rgba, width, height) = if used_background {
        crop_to_rgba(&capture, &selection.rect)?
    } else {
        bgra_to_rgba_full(&capture)
    };
    save_png(&save_path, &rgba, width, height)?;
    copy_to_clipboard(&rgba, width, height).ok();
    Ok(())
}

pub fn capture_active_output(config: &config::Config) -> Result<()> {
    let target = capture_area(false)?;
    let capture = capture_screen_gdi(&target.rect)?;
    let save_dir = config::get_screenshots_dir(None, config, false)?;
    let save_dir = config::ensure_directory(&save_dir.to_string_lossy())?;
    let filename = default_filename();
    let save_path = save_dir.join(filename);
    let (rgba, width, height) = bgra_to_rgba_full(&capture);
    save_png(&save_path, &rgba, width, height)?;
    copy_to_clipboard(&rgba, width, height).ok();
    Ok(())
}

#[derive(Clone)]
pub(crate) struct ScreenCapture {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) pixels: Vec<u8>, // BGRA top-down
}

#[derive(Clone, Copy)]
pub(crate) struct CaptureRect {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
}

#[derive(Clone, Copy)]
struct WindowSelection {
    hwnd: HWND,
    rect: CaptureRect,
}

#[derive(Clone, Copy)]
struct MonitorSelection {
    monitor: Option<HMONITOR>,
    rect: CaptureRect,
}

impl CaptureRect {
    fn width(self) -> i32 {
        self.right - self.left
    }
    fn height(self) -> i32 {
        self.bottom - self.top
    }
}

struct CaptureTarget {
    rect: CaptureRect,
    monitor: Option<HMONITOR>,
}

fn capture_area(all_monitors: bool) -> Result<CaptureTarget> {
    unsafe {
        if all_monitors {
            let left = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let top = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);
            return Ok(CaptureTarget {
                rect: CaptureRect {
                    left,
                    top,
                    right: left + width,
                    bottom: top + height,
                },
                monitor: None,
            });
        }
        let mut pt = POINT::default();
        if GetCursorPos(&mut pt).is_err() {
            anyhow::bail!("Failed to get cursor position");
        }
        let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetMonitorInfoW(monitor, &mut info)
            .as_bool()
            .then_some(())
            .context("Failed to get monitor information")?;

        Ok(CaptureTarget {
            rect: CaptureRect {
                left: info.rcMonitor.left,
                top: info.rcMonitor.top,
                right: info.rcMonitor.right,
                bottom: info.rcMonitor.bottom,
            },
            monitor: Some(monitor),
        })
    }
}

fn capture_screen(target: &CaptureTarget) -> Result<ScreenCapture> {
    if let Some(monitor) = target.monitor {
        if let Ok(capture) = capture_screen_wgc(monitor, &target.rect) {
            return Ok(capture);
        }
    }
    capture_screen_gdi(&target.rect)
}

fn capture_screen_gdi(rect: &CaptureRect) -> Result<ScreenCapture> {
    unsafe {
        let screen_dc = GetDC(HWND(0));
        if screen_dc.0 == 0 {
            anyhow::bail!("Failed to get screen DC");
        }
        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc.0 == 0 {
            ReleaseDC(HWND(0), screen_dc);
            anyhow::bail!("Failed to create DC");
        }
        let width = rect.width();
        let height = rect.height();
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: (width * height * 4) as u32,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits_ptr = null_mut();
        let dib = CreateDIBSection(screen_dc, &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0)
            .context("Failed to create DIB")?;
        if dib.0 == 0 {
            DeleteDC(mem_dc);
            ReleaseDC(HWND(0), screen_dc);
            anyhow::bail!("Failed to create DIB");
        }
        let old = SelectObject(mem_dc, HGDIOBJ(dib.0));
        let ok = BitBlt(
            mem_dc,
            0,
            0,
            width,
            height,
            screen_dc,
            rect.left,
            rect.top,
            SRCCOPY | CAPTUREBLT,
        )
        .is_ok();
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        if ok && !bits_ptr.is_null() {
            let src = std::slice::from_raw_parts(bits_ptr as *const u8, pixels.len());
            pixels.copy_from_slice(src);
        } else {
            SelectObject(mem_dc, old);
            DeleteObject(HGDIOBJ(dib.0));
            DeleteDC(mem_dc);
            ReleaseDC(HWND(0), screen_dc);
            anyhow::bail!("Failed to capture screen");
        }
        SelectObject(mem_dc, old);
        DeleteObject(HGDIOBJ(dib.0));
        DeleteDC(mem_dc);
        ReleaseDC(HWND(0), screen_dc);
        Ok(ScreenCapture {
            left: rect.left,
            top: rect.top,
            width,
            height,
            pixels,
        })
    }
}

fn capture_screen_wgc(monitor: HMONITOR, rect: &CaptureRect) -> Result<ScreenCapture> {
    let ro_inited = unsafe { RoInitialize(RO_INIT_MULTITHREADED).is_ok() };
    let (device, context, winrt_device) = create_d3d_device()?;
    let item = create_item_for_monitor(monitor)?;
    let size = item.Size()?;
    let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &winrt_device,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        1,
        SizeInt32 {
            Width: size.Width,
            Height: size.Height,
        },
    )?;
    let session = pool.CreateCaptureSession(&item)?;
    session.StartCapture()?;
    let frame = get_next_frame(&pool)?;
    let surface = frame.Surface()?;
    let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
    let texture: ID3D11Texture2D = unsafe { access.GetInterface()? };
    let capture = copy_texture_to_cpu(&device, &context, &texture, rect)?;
    let _ = frame.Close();
    let _ = pool.Close();
    let _ = session.Close();
    if ro_inited {
        unsafe {
            RoUninitialize();
        }
    }
    Ok(capture)
}

fn capture_window_print(hwnd: HWND) -> Result<ScreenCapture> {
    unsafe {
        let rect = window_rect(hwnd).context("Failed to get window rect")?;
        let width = rect.width();
        let height = rect.height();
        if width <= 0 || height <= 0 {
            anyhow::bail!("Invalid window size");
        }
        let screen_dc = GetDC(HWND(0));
        if screen_dc.0 == 0 {
            anyhow::bail!("Failed to get screen DC");
        }
        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc.0 == 0 {
            ReleaseDC(HWND(0), screen_dc);
            anyhow::bail!("Failed to create DC");
        }
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: (width * height * 4) as u32,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits_ptr = null_mut();
        let dib = CreateDIBSection(screen_dc, &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0)
            .context("Failed to create DIB")?;
        if dib.0 == 0 {
            DeleteDC(mem_dc);
            ReleaseDC(HWND(0), screen_dc);
            anyhow::bail!("Failed to create DIB");
        }
        let old = SelectObject(mem_dc, HGDIOBJ(dib.0));
        let ok = PrintWindow(hwnd, mem_dc, PW_RENDERFULLCONTENT).as_bool();
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        if ok && !bits_ptr.is_null() {
            let src = std::slice::from_raw_parts(bits_ptr as *const u8, pixels.len());
            pixels.copy_from_slice(src);
        } else {
            SelectObject(mem_dc, old);
            DeleteObject(HGDIOBJ(dib.0));
            DeleteDC(mem_dc);
            ReleaseDC(HWND(0), screen_dc);
            anyhow::bail!("Failed to capture window via PrintWindow");
        }
        SelectObject(mem_dc, old);
        DeleteObject(HGDIOBJ(dib.0));
        DeleteDC(mem_dc);
        ReleaseDC(HWND(0), screen_dc);
        Ok(ScreenCapture {
            left: rect.left,
            top: rect.top,
            width,
            height,
            pixels,
        })
    }
}

fn create_d3d_device() -> Result<(ID3D11Device, ID3D11DeviceContext, IDirect3DDevice)> {
    unsafe {
        let mut device: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            None,
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(&[D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0]),
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )?;
        let device = device.context("Failed to create D3D device")?;
        let context = context.context("Failed to create D3D context")?;
        let dxgi: IDXGIDevice = device.cast()?;
        let inspectable = CreateDirect3D11DeviceFromDXGIDevice(&dxgi)?;
        let winrt_device: IDirect3DDevice = inspectable.cast()?;
        Ok((device, context, winrt_device))
    }
}

fn create_item_for_monitor(monitor: HMONITOR) -> Result<GraphicsCaptureItem> {
    let interop: IGraphicsCaptureItemInterop =
        windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    let item: GraphicsCaptureItem = unsafe { interop.CreateForMonitor(monitor)? };
    Ok(item)
}

fn get_next_frame(
    pool: &Direct3D11CaptureFramePool,
) -> Result<windows::Graphics::Capture::Direct3D11CaptureFrame> {
    for _ in 0..50 {
        if let Ok(frame) = pool.TryGetNextFrame() {
            return Ok(frame);
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    anyhow::bail!("Failed to get frame via WGC");
}

fn copy_texture_to_cpu(
    device: &ID3D11Device,
    context: &ID3D11DeviceContext,
    texture: &ID3D11Texture2D,
    rect: &CaptureRect,
) -> Result<ScreenCapture> {
    unsafe {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        texture.GetDesc(&mut desc);
        let mut staging_desc = desc;
        staging_desc.Usage = D3D11_USAGE_STAGING;
        staging_desc.BindFlags = 0;
        staging_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        staging_desc.MiscFlags = 0;
        let mut staging: Option<ID3D11Texture2D> = None;
        device.CreateTexture2D(&staging_desc, None, Some(&mut staging))?;
        let staging = staging.context("Failed to create staging texture")?;
        let dst: ID3D11Resource = staging.cast()?;
        let src: ID3D11Resource = texture.cast()?;
        context.CopyResource(Some(&dst), Some(&src));
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        context.Map(Some(&dst), 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;
        let width = desc.Width as usize;
        let height = desc.Height as usize;
        let row_pitch = mapped.RowPitch as usize;
        let mut pixels = vec![0u8; width * height * 4];
        let src_ptr = mapped.pData as *const u8;
        for row in 0..height {
            let src_row = src_ptr.add(row * row_pitch);
            let dst_row = &mut pixels[row * width * 4..(row + 1) * width * 4];
            std::ptr::copy_nonoverlapping(src_row, dst_row.as_mut_ptr(), width * 4);
        }
        context.Unmap(Some(&dst), 0);
        Ok(ScreenCapture {
            left: rect.left,
            top: rect.top,
            width: width as i32,
            height: height as i32,
            pixels,
        })
    }
}

fn capture_window_wgc(hwnd: HWND, rect: CaptureRect) -> Result<ScreenCapture> {
    let ro_inited = unsafe { RoInitialize(RO_INIT_MULTITHREADED).is_ok() };
    let (device, context, winrt_device) = create_d3d_device()?;
    let item = create_item_for_window(hwnd)?;
    let size = item.Size()?;
    let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &winrt_device,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        1,
        SizeInt32 {
            Width: size.Width,
            Height: size.Height,
        },
    )?;
    let session = pool.CreateCaptureSession(&item)?;
    session.StartCapture()?;
    let frame = get_next_frame(&pool)?;
    let surface = frame.Surface()?;
    let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
    let texture: ID3D11Texture2D = unsafe { access.GetInterface()? };
    let mut capture = copy_texture_to_cpu(&device, &context, &texture, &rect)?;
    capture.left = rect.left;
    capture.top = rect.top;
    let _ = frame.Close();
    let _ = pool.Close();
    let _ = session.Close();
    if ro_inited {
        unsafe {
            RoUninitialize();
        }
    }
    Ok(capture)
}

fn create_item_for_window(hwnd: HWND) -> Result<GraphicsCaptureItem> {
    let interop: IGraphicsCaptureItemInterop =
        windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(hwnd)? };
    Ok(item)
}

fn bgra_to_rgba_full(capture: &ScreenCapture) -> (Vec<u8>, u32, u32) {
    let width = capture.width as usize;
    let height = capture.height as usize;
    let mut rgba = vec![0u8; width * height * 4];
    for i in 0..(width * height) {
        let si = i * 4;
        rgba[si] = capture.pixels[si + 2];
        rgba[si + 1] = capture.pixels[si + 1];
        rgba[si + 2] = capture.pixels[si];
        rgba[si + 3] = capture.pixels[si + 3];
    }
    (rgba, capture.width as u32, capture.height as u32)
}

fn rgb_color(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

#[derive(Default)]
struct OverlayState {
    capture: Option<ScreenCapture>,
    dimmed: Option<Vec<u8>>,
    buffer: Option<OverlayBuffer>,
    selecting: bool,
    start: POINT,
    current: POINT,
    last_rect: Option<RECT>,
    last_label_rect: Option<RECT>,
    done: bool,
    canceled: bool,
    result: Option<CaptureRect>,
}

static OVERLAY_STATE: std::sync::OnceLock<std::sync::Mutex<OverlayState>> =
    std::sync::OnceLock::new();

#[derive(Default)]
struct WindowOverlayState {
    capture: Option<ScreenCapture>,
    dimmed: Option<Vec<u8>>,
    buffer: Option<OverlayBuffer>,
    current_hwnd: Option<HWND>,
    current_rect: Option<CaptureRect>,
    last_rect: Option<RECT>,
    last_raise_hwnd: Option<HWND>,
    last_raise_was_topmost: Option<bool>,
    cursor: POINT,
    done: bool,
    canceled: bool,
    result: Option<WindowSelection>,
    overlay_hwnd: HWND,
}

static WINDOW_OVERLAY_STATE: std::sync::OnceLock<std::sync::Mutex<WindowOverlayState>> =
    std::sync::OnceLock::new();

#[derive(Default)]
struct MonitorOverlayState {
    capture: Option<ScreenCapture>,
    dimmed: Option<Vec<u8>>,
    buffer: Option<OverlayBuffer>,
    current_rect: Option<CaptureRect>,
    current_monitor: Option<HMONITOR>,
    cursor: POINT,
    done: bool,
    canceled: bool,
    result: Option<MonitorSelection>,
    overlay_hwnd: HWND,
}

static MONITOR_OVERLAY_STATE: std::sync::OnceLock<std::sync::Mutex<MonitorOverlayState>> =
    std::sync::OnceLock::new();

const OVERLAY_DIM_ALPHA: f32 = 0.55;
const WINDOW_OVERLAY_DOUBLE_BUFFER: bool = true;
const WINDOW_CAPTURE_USE_WGC: bool = false;
const MONITOR_CAPTURE_USE_WGC: bool = false;
const PW_RENDERFULLCONTENT: u32 = 0x00000002;

#[link(name = "user32")]
unsafe extern "system" {
    fn PrintWindow(hwnd: HWND, hdc: HDC, nflags: u32) -> BOOL;
}

struct OverlayBuffer {
    dc: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    width: i32,
    height: i32,
}

impl OverlayBuffer {
    unsafe fn new(hwnd: HWND, width: i32, height: i32) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }
        let screen_dc = GetDC(hwnd);
        if screen_dc.0 == 0 {
            return None;
        }
        let mem_dc = CreateCompatibleDC(screen_dc);
        ReleaseDC(hwnd, screen_dc);
        if mem_dc.0 == 0 {
            return None;
        }
        let mut bits_ptr = null_mut();
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let bmp = match CreateDIBSection(mem_dc, &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0) {
            Ok(bmp) => bmp,
            Err(_) => {
                DeleteDC(mem_dc);
                return None;
            }
        };
        if bmp.0 == 0 {
            DeleteDC(mem_dc);
            return None;
        }
        let old = SelectObject(mem_dc, HGDIOBJ(bmp.0));
        Some(Self {
            dc: mem_dc,
            bmp,
            old,
            width,
            height,
        })
    }

    unsafe fn destroy(&mut self) {
        let _ = SelectObject(self.dc, self.old);
        DeleteObject(HGDIOBJ(self.bmp.0));
        DeleteDC(self.dc);
    }
}

unsafe fn ensure_overlay_buffer(
    buffer: &mut Option<OverlayBuffer>,
    hwnd: HWND,
    width: i32,
    height: i32,
) -> Option<&mut OverlayBuffer> {
    let recreate = match buffer {
        Some(buf) => buf.width != width || buf.height != height,
        None => true,
    };
    if recreate {
        if let Some(mut old) = buffer.take() {
            old.destroy();
        }
        *buffer = OverlayBuffer::new(hwnd, width, height);
    }
    buffer.as_mut()
}

unsafe fn destroy_overlay_buffer(buffer: &mut Option<OverlayBuffer>) {
    if let Some(mut buf) = buffer.take() {
        buf.destroy();
    }
}

pub(crate) fn dim_pixels(pixels: &[u8]) -> Vec<u8> {
    let mut out = pixels.to_vec();
    let factor = (1.0 - OVERLAY_DIM_ALPHA).clamp(0.0, 1.0);
    for px in out.chunks_exact_mut(4) {
        px[0] = (px[0] as f32 * factor) as u8;
        px[1] = (px[1] as f32 * factor) as u8;
        px[2] = (px[2] as f32 * factor) as u8;
    }
    out
}

fn run_region_overlay(capture: ScreenCapture) -> Result<Option<CaptureRect>> {
    let state = OVERLAY_STATE.get_or_init(|| std::sync::Mutex::new(OverlayState::default()));
    {
        let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
        let dimmed = dim_pixels(&capture.pixels);
        *guard = OverlayState {
            capture: Some(capture),
            dimmed: Some(dimmed),
            buffer: None,
            selecting: false,
            start: POINT::default(),
            current: POINT::default(),
            last_rect: None,
            last_label_rect: None,
            done: false,
            canceled: false,
            result: None,
        };
    }
    unsafe {
        let hinstance: HINSTANCE =
            windows::Win32::System::LibraryLoader::GetModuleHandleW(None)?.into();
        let class_name = w!("HyprshotOverlayWindow");
        let wnd = WNDCLASSW {
            lpfnWndProc: Some(overlay_wnd_proc),
            hInstance: hinstance,
            lpszClassName: class_name,
            style: CS_HREDRAW | CS_VREDRAW,
            ..Default::default()
        };
        RegisterClassW(&wnd);
        let (left, top, width, height) = with_overlay_state(|s| {
            let c = s.capture.as_ref().unwrap();
            (c.left, c.top, c.width, c.height)
        })
        .unwrap_or((0, 0, 0, 0));
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT,
            class_name,
            w!(""),
            WS_POPUP | WS_VISIBLE,
            left,
            top,
            width,
            height,
            HWND(0),
            HMENU(0),
            hinstance,
            None,
        );
        if hwnd.0 == 0 {
            return Ok(None);
        }
        SetForegroundWindow(hwnd);
        let mut msg = MSG::default();
        loop {
            while PeekMessageW(&mut msg, HWND(0), 0, 0, PM_REMOVE).as_bool() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let done = with_overlay_state(|s| s.done).unwrap_or(true);
            if done {
                break;
            }
            if escape_pressed() {
                with_overlay_state(|s| {
                    s.canceled = true;
                    s.done = true;
                });
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let result = with_overlay_state(|s| {
            let output = if s.canceled { None } else { s.result };
            s.capture = None;
            destroy_overlay_buffer(&mut s.buffer);
            output
        })
        .unwrap_or(None);
        let _ = DestroyWindow(hwnd);
        Ok(result)
    }
}

fn run_window_overlay(capture: ScreenCapture) -> Result<Option<WindowSelection>> {
    let state =
        WINDOW_OVERLAY_STATE.get_or_init(|| std::sync::Mutex::new(WindowOverlayState::default()));
    {
        let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
        *guard = WindowOverlayState {
            capture: Some(capture),
            dimmed: None,
            buffer: None,
            current_hwnd: None,
            current_rect: None,
            last_rect: None,
            last_raise_hwnd: None,
            last_raise_was_topmost: None,
            cursor: POINT::default(),
            done: false,
            canceled: false,
            result: None,
            overlay_hwnd: HWND(0),
        };
    }
    unsafe {
        let hinstance: HINSTANCE =
            windows::Win32::System::LibraryLoader::GetModuleHandleW(None)?.into();
        let class_name = w!("HyprshotWindowOverlay");
        let wnd = WNDCLASSW {
            lpfnWndProc: Some(window_overlay_wnd_proc),
            hInstance: hinstance,
            lpszClassName: class_name,
            style: CS_HREDRAW | CS_VREDRAW,
            ..Default::default()
        };
        RegisterClassW(&wnd);
        let (left, top, width, height) = with_window_overlay_state(|s| {
            let c = s.capture.as_ref().unwrap();
            (c.left, c.top, c.width, c.height)
        })
        .unwrap_or((0, 0, 0, 0));
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name,
            w!(""),
            WS_POPUP | WS_VISIBLE,
            left,
            top,
            width,
            height,
            HWND(0),
            HMENU(0),
            hinstance,
            None,
        );
        if hwnd.0 == 0 {
            return Ok(None);
        }
        with_window_overlay_state(|s| s.overlay_hwnd = hwnd);
        SetForegroundWindow(hwnd);
        let mut msg = MSG::default();
        loop {
            while PeekMessageW(&mut msg, HWND(0), 0, 0, PM_REMOVE).as_bool() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let done = with_window_overlay_state(|s| s.done).unwrap_or(true);
            if done {
                break;
            }
            if escape_pressed() {
                with_window_overlay_state(|s| {
                    s.canceled = true;
                    s.done = true;
                });
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let result = with_window_overlay_state(|s| {
            let output = if s.canceled { None } else { s.result };
            if let (Some(prev), Some(prev_topmost)) = (s.last_raise_hwnd, s.last_raise_was_topmost)
            {
                set_window_topmost(prev, prev_topmost);
            }
            s.last_raise_hwnd = None;
            s.last_raise_was_topmost = None;
            s.capture = None;
            destroy_overlay_buffer(&mut s.buffer);
            output
        })
        .unwrap_or(None);
        let _ = DestroyWindow(hwnd);
        Ok(result)
    }
}

unsafe extern "system" fn overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            set_cross_cursor();
            SetCapture(hwnd);
            LRESULT(0)
        }
        WM_SETCURSOR => {
            set_cross_cursor();
            LRESULT(1)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_LBUTTONDOWN => {
            let pt = lparam_point(lparam);
            with_overlay_state(|s| {
                s.selecting = true;
                s.start = pt;
                s.current = pt;
            });
            SetCapture(hwnd);
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let pt = lparam_point(lparam);
            let invalidate_rect = with_overlay_state(|s| {
                if !s.selecting {
                    return None;
                }
                let old_rect = s.last_rect;
                let old_label = s.last_label_rect;
                s.current = pt;
                let new_rect = selection_rect_client(s);
                s.last_rect = new_rect;

                let Some(capture) = s.capture.as_ref() else {
                    return None;
                };
                let mut dirty = None;
                if let Some(old) = old_rect {
                    dirty = Some(old);
                }
                if let Some(newr) = new_rect {
                    dirty = Some(dirty.map(|d| union_rect(d, newr)).unwrap_or(newr));
                }
                let label = label_rect_at(pt, capture.width, capture.height);
                s.last_label_rect = Some(label);
                if let Some(old) = old_label {
                    dirty = Some(dirty.map(|d| union_rect(d, old)).unwrap_or(old));
                }
                dirty = Some(dirty.map(|d| union_rect(d, label)).unwrap_or(label));
                dirty = dirty.map(|d| inflate_rect(d, 6));
                dirty
            });
            if let Some(rect) = invalidate_rect.flatten() {
                let _ = InvalidateRect(hwnd, Some(&rect), BOOL(0));
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let pt = lparam_point(lparam);
            with_overlay_state(|s| {
                s.current = pt;
                s.selecting = false;
                s.result = selection_rect(s);
                s.done = true;
            });
            let _ = ReleaseCapture();
            LRESULT(0)
        }
        WM_RBUTTONDOWN | WM_RBUTTONUP | WM_KEYDOWN => {
            if msg == WM_KEYDOWN && wparam.0 as u32 != 0x1B {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            with_overlay_state(|s| {
                s.canceled = true;
                s.done = true;
            });
            let _ = ReleaseCapture();
            LRESULT(0)
        }
        WM_PAINT => {
            paint_overlay(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => LRESULT(0),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe extern "system" fn window_overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            set_cross_cursor();
            LRESULT(0)
        }
        WM_SETCURSOR => {
            set_cross_cursor();
            LRESULT(1)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_MOUSEMOVE => {
            let client = lparam_point(lparam);
            with_window_overlay_state(|s| s.cursor = client);
            let screen_pt = window_overlay_to_screen(client);
            let invalidate_rect =
                if let Some(hwnd_found) = window_from_point_skip_overlay(hwnd, screen_pt) {
                    let rect = window_rect(hwnd_found);
                    with_window_overlay_state(|s| {
                        let old_rect = s.last_rect;
                        s.current_hwnd = Some(hwnd_found);
                        s.current_rect = rect;

                        if s.last_raise_hwnd != Some(hwnd_found) {
                            if let (Some(prev), Some(prev_topmost)) =
                                (s.last_raise_hwnd, s.last_raise_was_topmost)
                            {
                                set_window_topmost(prev, prev_topmost);
                            }
                            let was_topmost = is_window_topmost(hwnd_found);
                            set_window_topmost(hwnd_found, true);
                            s.last_raise_hwnd = Some(hwnd_found);
                            s.last_raise_was_topmost = Some(was_topmost);
                            if let Some(current_capture) = s.capture.as_ref() {
                                std::thread::sleep(std::time::Duration::from_millis(20));
                                let rect = CaptureRect {
                                    left: current_capture.left,
                                    top: current_capture.top,
                                    right: current_capture.left + current_capture.width,
                                    bottom: current_capture.top + current_capture.height,
                                };
                                if s.overlay_hwnd.0 != 0 {
                                    ShowWindow(s.overlay_hwnd, SW_HIDE);
                                }
                                unsafe {
                                    let _ = DwmFlush();
                                }
                                if let Ok(updated) = capture_screen_gdi(&rect) {
                                    s.capture = Some(updated);
                                }
                                if s.overlay_hwnd.0 != 0 {
                                    ShowWindow(s.overlay_hwnd, SW_SHOW);
                                }
                            }
                        }
                        let new_rect = s.current_rect.and_then(|r| {
                            let capture = s.capture.as_ref()?;
                            let clamped = clamp_rect_to_bounds(
                                r,
                                capture.left,
                                capture.top,
                                capture.width,
                                capture.height,
                            );
                            Some(RECT {
                                left: clamped.left - capture.left,
                                top: clamped.top - capture.top,
                                right: clamped.right - capture.left,
                                bottom: clamped.bottom - capture.top,
                            })
                        });
                        s.last_rect = new_rect;
                        let dirty = match (old_rect, new_rect) {
                            (Some(a), Some(b)) => Some(union_rect(a, b)),
                            (Some(a), None) => Some(a),
                            (None, Some(b)) => Some(b),
                            (None, None) => None,
                        };
                        dirty.map(|d| inflate_rect(d, 8))
                    })
                } else {
                    with_window_overlay_state(|s| {
                        let old_rect = s.last_rect;
                        if let (Some(prev), Some(prev_topmost)) =
                            (s.last_raise_hwnd, s.last_raise_was_topmost)
                        {
                            set_window_topmost(prev, prev_topmost);
                        }
                        s.last_raise_hwnd = None;
                        s.last_raise_was_topmost = None;
                        s.current_hwnd = None;
                        s.current_rect = None;
                        s.last_rect = None;
                        old_rect.map(|d| inflate_rect(d, 8))
                    })
                };
            if let Some(rect) = invalidate_rect.flatten() {
                let _ = InvalidateRect(hwnd, Some(&rect), BOOL(0));
            } else {
                invalidate(hwnd);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let selection = with_window_overlay_state(|s| {
                let rect = s.current_rect;
                let hwnd = s.current_hwnd;
                if let (Some(hwnd), Some(rect)) = (hwnd, rect) {
                    s.result = Some(WindowSelection { hwnd, rect });
                }
                s.done = true;
                s.result
            })
            .unwrap_or(None);
            if selection.is_none() {
                with_window_overlay_state(|s| s.canceled = true);
            }
            let _ = ReleaseCapture();
            LRESULT(0)
        }
        WM_RBUTTONDOWN | WM_RBUTTONUP | WM_KEYDOWN => {
            if msg == WM_KEYDOWN && wparam.0 as u32 != 0x1B {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            with_window_overlay_state(|s| {
                s.canceled = true;
                s.done = true;
            });
            let _ = ReleaseCapture();
            LRESULT(0)
        }
        WM_PAINT => {
            paint_window_overlay(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            let _ = ReleaseCapture();
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn paint_overlay(hwnd: HWND) {
    unsafe {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        if hdc.0 == 0 {
            return;
        }
        let Some(state_lock) = OVERLAY_STATE.get() else {
            EndPaint(hwnd, &ps);
            return;
        };
        let mut guard = state_lock.lock().unwrap_or_else(|e| e.into_inner());
        let (
            cap_width,
            cap_height,
            cap_left,
            cap_top,
            dimmed_ptr,
            capture_ptr,
            selecting,
            has_result,
            rect_opt,
            current_pt,
        ) = {
            let Some(capture) = guard.capture.as_ref() else {
                EndPaint(hwnd, &ps);
                return;
            };
            let dimmed = guard.dimmed.as_deref().unwrap_or(&capture.pixels);
            let rect = if guard.selecting || guard.result.is_some() {
                selection_rect(&guard)
            } else {
                None
            };
            (
                capture.width,
                capture.height,
                capture.left,
                capture.top,
                dimmed.as_ptr(),
                capture.pixels.as_ptr(),
                guard.selecting,
                guard.result.is_some(),
                rect,
                guard.current,
            )
        };
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: cap_width,
                biHeight: -cap_height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let Some(buffer) = ensure_overlay_buffer(&mut guard.buffer, hwnd, cap_width, cap_height)
        else {
            EndPaint(hwnd, &ps);
            return;
        };
        let dirty = ps.rcPaint;
        let dirty_w = dirty.right - dirty.left;
        let dirty_h = dirty.bottom - dirty.top;
        if dirty_w <= 0 || dirty_h <= 0 {
            EndPaint(hwnd, &ps);
            return;
        }
        let dirty_rgn = CreateRectRgn(dirty.left, dirty.top, dirty.right, dirty.bottom);
        if dirty_rgn.0 != 0 {
            let _ = SelectClipRgn(buffer.dc, dirty_rgn);
        }
        let _ = windows::Win32::Graphics::Gdi::StretchDIBits(
            buffer.dc,
            0,
            0,
            cap_width,
            cap_height,
            0,
            0,
            cap_width,
            cap_height,
            Some(dimmed_ptr as *const _),
            &mut bmi,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
        if selecting || has_result {
            if let Some(mut r) = rect_opt {
                r = clamp_rect_to_bounds(r, cap_left, cap_top, cap_width, cap_height);
                let width = r.right - r.left;
                let height = r.bottom - r.top;
                if width > 0 && height > 0 {
                    let sx = r.left - cap_left;
                    let sy = r.top - cap_top;
                    let rgn = CreateRectRgn(sx, sy, sx + width, sy + height);
                    if rgn.0 != 0 {
                        let _ = SelectClipRgn(buffer.dc, rgn);
                        let _ = windows::Win32::Graphics::Gdi::StretchDIBits(
                            buffer.dc,
                            0,
                            0,
                            cap_width,
                            cap_height,
                            0,
                            0,
                            cap_width,
                            cap_height,
                            Some(capture_ptr as *const _),
                            &mut bmi,
                            DIB_RGB_COLORS,
                            SRCCOPY,
                        );
                        if dirty_rgn.0 != 0 {
                            let _ = SelectClipRgn(buffer.dc, dirty_rgn);
                        }
                        DeleteObject(HGDIOBJ(rgn.0));
                    }
                }
                if dirty_rgn.0 != 0 {
                    let _ = SelectClipRgn(buffer.dc, dirty_rgn);
                }
                let pen = CreatePen(PS_SOLID, 2, rgb_color(255, 255, 255));
                let old_pen = SelectObject(buffer.dc, HGDIOBJ(pen.0));
                let null_brush = GetStockObject(NULL_BRUSH);
                let old_brush = SelectObject(buffer.dc, null_brush);
                windows::Win32::Graphics::Gdi::Rectangle(
                    buffer.dc,
                    r.left - cap_left,
                    r.top - cap_top,
                    r.right - cap_left,
                    r.bottom - cap_top,
                );
                SelectObject(buffer.dc, old_brush);
                SelectObject(buffer.dc, old_pen);
                DeleteObject(HGDIOBJ(pen.0));
                let label = format!("{} x {}", width, height);
                SetBkMode(buffer.dc, TRANSPARENT);
                SetTextColor(buffer.dc, rgb_color(255, 255, 255));
                let pos = current_pt;
                let wide: Vec<u16> = label.encode_utf16().collect();
                let _ = TextOutW(buffer.dc, pos.x + 12, pos.y + 12, &wide);
            }
        }
        let _ = SelectClipRgn(buffer.dc, HRGN(0));
        if dirty_rgn.0 != 0 {
            DeleteObject(HGDIOBJ(dirty_rgn.0));
        }
        let _ = BitBlt(
            hdc, dirty.left, dirty.top, dirty_w, dirty_h, buffer.dc, dirty.left, dirty.top, SRCCOPY,
        );
        EndPaint(hwnd, &ps);
    }
}

fn paint_window_overlay(hwnd: HWND) {
    unsafe {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        if hdc.0 == 0 {
            return;
        }
        let Some(state_lock) = WINDOW_OVERLAY_STATE.get() else {
            EndPaint(hwnd, &ps);
            return;
        };
        let mut guard = state_lock.lock().unwrap_or_else(|e| e.into_inner());
        let (cap_width, cap_height, cap_left, cap_top, dimmed_ptr, capture_ptr, rect_opt) = {
            let Some(capture) = guard.capture.as_ref() else {
                EndPaint(hwnd, &ps);
                return;
            };
            let dimmed = guard.dimmed.as_deref().unwrap_or(&capture.pixels);
            (
                capture.width,
                capture.height,
                capture.left,
                capture.top,
                dimmed.as_ptr(),
                capture.pixels.as_ptr(),
                guard.current_rect,
            )
        };
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: cap_width,
                biHeight: -cap_height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        if WINDOW_OVERLAY_DOUBLE_BUFFER {
            let Some(buffer) =
                ensure_overlay_buffer(&mut guard.buffer, hwnd, cap_width, cap_height)
            else {
                EndPaint(hwnd, &ps);
                return;
            };
            let dirty = ps.rcPaint;
            let dirty_w = dirty.right - dirty.left;
            let dirty_h = dirty.bottom - dirty.top;
            if dirty_w <= 0 || dirty_h <= 0 {
                EndPaint(hwnd, &ps);
                return;
            }
            let dirty_rgn = CreateRectRgn(dirty.left, dirty.top, dirty.right, dirty.bottom);
            if dirty_rgn.0 != 0 {
                let _ = SelectClipRgn(buffer.dc, dirty_rgn);
            }
            let _ = windows::Win32::Graphics::Gdi::StretchDIBits(
                buffer.dc,
                0,
                0,
                cap_width,
                cap_height,
                0,
                0,
                cap_width,
                cap_height,
                Some(dimmed_ptr as *const _),
                &mut bmi,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            if let Some(mut rect) = rect_opt {
                rect = clamp_rect_to_bounds(rect, cap_left, cap_top, cap_width, cap_height);
                let width = rect.right - rect.left;
                let height = rect.bottom - rect.top;
                if width > 0 && height > 0 {
                    let sx = rect.left - cap_left;
                    let sy = rect.top - cap_top;
                    let rgn = CreateRectRgn(sx, sy, sx + width, sy + height);
                    if rgn.0 != 0 {
                        let _ = SelectClipRgn(buffer.dc, rgn);
                        let _ = windows::Win32::Graphics::Gdi::StretchDIBits(
                            buffer.dc,
                            0,
                            0,
                            cap_width,
                            cap_height,
                            0,
                            0,
                            cap_width,
                            cap_height,
                            Some(capture_ptr as *const _),
                            &mut bmi,
                            DIB_RGB_COLORS,
                            SRCCOPY,
                        );
                        if dirty_rgn.0 != 0 {
                            let _ = SelectClipRgn(buffer.dc, dirty_rgn);
                        }
                        DeleteObject(HGDIOBJ(rgn.0));
                    }
                }
                if dirty_rgn.0 != 0 {
                    let _ = SelectClipRgn(buffer.dc, dirty_rgn);
                }
                let null_brush = GetStockObject(NULL_BRUSH);
                let old_brush = SelectObject(buffer.dc, null_brush);
                let rect_l = rect.left - cap_left;
                let rect_t = rect.top - cap_top;
                let rect_r = rect.right - cap_left;
                let rect_b = rect.bottom - cap_top;
                for (width_px, color) in [
                    (6, rgb_color(200, 200, 200)),
                    (4, rgb_color(230, 230, 230)),
                    (2, rgb_color(255, 255, 255)),
                ] {
                    let pen = CreatePen(PS_SOLID, width_px, color);
                    let old_pen = SelectObject(buffer.dc, HGDIOBJ(pen.0));
                    windows::Win32::Graphics::Gdi::Rectangle(
                        buffer.dc, rect_l, rect_t, rect_r, rect_b,
                    );
                    SelectObject(buffer.dc, old_pen);
                    DeleteObject(HGDIOBJ(pen.0));
                }
                SelectObject(buffer.dc, old_brush);
            }
            let _ = SelectClipRgn(buffer.dc, HRGN(0));
            if dirty_rgn.0 != 0 {
                DeleteObject(HGDIOBJ(dirty_rgn.0));
            }
            let _ = BitBlt(
                hdc, dirty.left, dirty.top, dirty_w, dirty_h, buffer.dc, dirty.left, dirty.top,
                SRCCOPY,
            );
            EndPaint(hwnd, &ps);
            return;
        }
        let _ = windows::Win32::Graphics::Gdi::StretchDIBits(
            hdc,
            0,
            0,
            cap_width,
            cap_height,
            0,
            0,
            cap_width,
            cap_height,
            Some(dimmed_ptr as *const _),
            &mut bmi,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
        if let Some(mut rect) = rect_opt {
            rect = clamp_rect_to_bounds(rect, cap_left, cap_top, cap_width, cap_height);
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            if width > 0 && height > 0 {
                let sx = rect.left - cap_left;
                let sy = rect.top - cap_top;
                let rgn = CreateRectRgn(sx, sy, sx + width, sy + height);
                if rgn.0 != 0 {
                    let _ = SelectClipRgn(hdc, rgn);
                    let _ = windows::Win32::Graphics::Gdi::StretchDIBits(
                        hdc,
                        0,
                        0,
                        cap_width,
                        cap_height,
                        0,
                        0,
                        cap_width,
                        cap_height,
                        Some(capture_ptr as *const _),
                        &mut bmi,
                        DIB_RGB_COLORS,
                        SRCCOPY,
                    );
                    let _ = SelectClipRgn(hdc, HRGN(0));
                    DeleteObject(HGDIOBJ(rgn.0));
                }
            }
            let null_brush = GetStockObject(NULL_BRUSH);
            let old_brush = SelectObject(hdc, null_brush);
            let rect_l = rect.left - cap_left;
            let rect_t = rect.top - cap_top;
            let rect_r = rect.right - cap_left;
            let rect_b = rect.bottom - cap_top;
            for (width_px, color) in [
                (6, rgb_color(200, 200, 200)),
                (4, rgb_color(230, 230, 230)),
                (2, rgb_color(255, 255, 255)),
            ] {
                let pen = CreatePen(PS_SOLID, width_px, color);
                let old_pen = SelectObject(hdc, HGDIOBJ(pen.0));
                windows::Win32::Graphics::Gdi::Rectangle(hdc, rect_l, rect_t, rect_r, rect_b);
                SelectObject(hdc, old_pen);
                DeleteObject(HGDIOBJ(pen.0));
            }
            SelectObject(hdc, old_brush);
        }
        EndPaint(hwnd, &ps);
    }
}

fn selection_rect(state: &OverlayState) -> Option<CaptureRect> {
    if !state.selecting && state.result.is_some() {
        return state.result;
    }
    let start = state.start;
    let current = state.current;
    if start.x == 0 && start.y == 0 && current.x == 0 && current.y == 0 {
        return None;
    }
    let (mut left, mut right) = if start.x <= current.x {
        (start.x, current.x)
    } else {
        (current.x, start.x)
    };
    let (mut top, mut bottom) = if start.y <= current.y {
        (start.y, current.y)
    } else {
        (current.y, start.y)
    };
    if let Some(capture) = &state.capture {
        let max_x = capture.width;
        let max_y = capture.height;
        left = left.clamp(0, max_x);
        right = right.clamp(0, max_x);
        top = top.clamp(0, max_y);
        bottom = bottom.clamp(0, max_y);
    }
    if (right - left) < 2 || (bottom - top) < 2 {
        return None;
    }
    let (off_x, off_y) = state
        .capture
        .as_ref()
        .map(|c| (c.left, c.top))
        .unwrap_or((0, 0));

    Some(CaptureRect {
        left: left + off_x,
        top: top + off_y,
        right: right + off_x,
        bottom: bottom + off_y,
    })
}

fn invalidate(hwnd: HWND) {
    unsafe {
        let _ = InvalidateRect(hwnd, None, BOOL(0));
    }
}

fn escape_pressed() -> bool {
    unsafe { (GetAsyncKeyState(VK_ESCAPE.0 as i32) & (0x8000u16 as i16)) != 0 }
}

fn selection_rect_client(state: &OverlayState) -> Option<RECT> {
    let start = state.start;
    let current = state.current;
    if start.x == 0 && start.y == 0 && current.x == 0 && current.y == 0 {
        return None;
    }
    let (mut left, mut right) = if start.x <= current.x {
        (start.x, current.x)
    } else {
        (current.x, start.x)
    };
    let (mut top, mut bottom) = if start.y <= current.y {
        (start.y, current.y)
    } else {
        (current.y, start.y)
    };
    if let Some(capture) = &state.capture {
        let max_x = capture.width;
        let max_y = capture.height;
        left = left.clamp(0, max_x);
        right = right.clamp(0, max_x);
        top = top.clamp(0, max_y);
        bottom = bottom.clamp(0, max_y);
    }
    if (right - left) < 2 || (bottom - top) < 2 {
        return None;
    }
    Some(RECT {
        left,
        top,
        right,
        bottom,
    })
}

pub(crate) fn union_rect(a: RECT, b: RECT) -> RECT {
    RECT {
        left: a.left.min(b.left),
        top: a.top.min(b.top),
        right: a.right.max(b.right),
        bottom: a.bottom.max(b.bottom),
    }
}

pub(crate) fn inflate_rect(mut rect: RECT, pad: i32) -> RECT {
    rect.left -= pad;
    rect.top -= pad;
    rect.right += pad;
    rect.bottom += pad;
    rect
}

pub(crate) fn label_rect_at(point: POINT, max_w: i32, max_h: i32) -> RECT {
    let w = 100;
    let h = 24;
    let mut left = point.x + 12;
    let mut top = point.y + 12;
    let mut right = left + w;
    let mut bottom = top + h;
    if right > max_w {
        right = max_w;
        left = (right - w).max(0);
    }
    if bottom > max_h {
        bottom = max_h;
        top = (bottom - h).max(0);
    }
    RECT {
        left,
        top,
        right,
        bottom,
    }
}

pub(crate) fn lparam_point(lparam: LPARAM) -> POINT {
    let x = (lparam.0 & 0xFFFF) as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
    POINT { x, y }
}

fn set_cross_cursor() {
    unsafe {
        if let Ok(cursor) = LoadCursorW(None, IDC_CROSS) {
            let _ = SetCursor(cursor);
        }
    }
}

fn with_overlay_state<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut OverlayState) -> R,
{
    OVERLAY_STATE.get().map(|state| {
        let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut guard)
    })
}

fn with_window_overlay_state<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut WindowOverlayState) -> R,
{
    WINDOW_OVERLAY_STATE.get().map(|state| {
        let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut guard)
    })
}

fn with_monitor_overlay_state<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut MonitorOverlayState) -> R,
{
    MONITOR_OVERLAY_STATE.get().map(|state| {
        let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut guard)
    })
}

fn monitor_overlay_to_screen(client: POINT) -> POINT {
    with_monitor_overlay_state(|s| {
        let offset = s
            .capture
            .as_ref()
            .map(|c| (c.left, c.top))
            .unwrap_or((0, 0));
        POINT {
            x: client.x + offset.0,
            y: client.y + offset.1,
        }
    })
    .unwrap_or(client)
}

fn monitor_rect_at_point(pt: POINT) -> Option<(HMONITOR, CaptureRect)> {
    unsafe {
        let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        if monitor.0 == 0 {
            return None;
        }
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return None;
        }
        Some((
            monitor,
            CaptureRect {
                left: info.rcMonitor.left,
                top: info.rcMonitor.top,
                right: info.rcMonitor.right,
                bottom: info.rcMonitor.bottom,
            },
        ))
    }
}

fn window_overlay_to_screen(client: POINT) -> POINT {
    with_window_overlay_state(|s| {
        let offset = s
            .capture
            .as_ref()
            .map(|c| (c.left, c.top))
            .unwrap_or((0, 0));
        POINT {
            x: client.x + offset.0,
            y: client.y + offset.1,
        }
    })
    .unwrap_or(client)
}

fn window_from_point_skip_overlay(overlay_hwnd: HWND, pt: POINT) -> Option<HWND> {
    unsafe {
        let mut hwnd = WindowFromPoint(pt);
        let mut attempts = 0;
        while hwnd.0 != 0 && attempts < 64 {
            let root = GetAncestor(hwnd, GA_ROOT);
            if root.0 == 0 {
                hwnd = GetWindow(hwnd, GW_HWNDNEXT);
                attempts += 1;
                continue;
            }
            if root == overlay_hwnd {
                hwnd = GetWindow(hwnd, GW_HWNDNEXT);
                attempts += 1;
                continue;
            }
            if is_valid_capture_window(root) {
                if let Some(rect) = window_rect(root) {
                    if point_in_rect(pt, rect) {
                        return Some(root);
                    }
                }
            }
            hwnd = GetWindow(root, GW_HWNDNEXT);
            attempts += 1;
        }
        None
    }
}

fn paint_monitor_overlay(hwnd: HWND) {
    unsafe {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        if hdc.0 == 0 {
            return;
        }
        let Some(state_lock) = MONITOR_OVERLAY_STATE.get() else {
            EndPaint(hwnd, &ps);
            return;
        };
        let mut guard = state_lock.lock().unwrap_or_else(|e| e.into_inner());
        let (cap_width, cap_height, cap_left, cap_top, dimmed_ptr, capture_ptr, rect_opt, cursor) = {
            let Some(capture) = guard.capture.as_ref() else {
                EndPaint(hwnd, &ps);
                return;
            };
            let dimmed = guard.dimmed.as_deref().unwrap_or(&capture.pixels);
            (
                capture.width,
                capture.height,
                capture.left,
                capture.top,
                dimmed.as_ptr(),
                capture.pixels.as_ptr(),
                guard.current_rect,
                guard.cursor,
            )
        };
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: cap_width,
                biHeight: -cap_height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let Some(buffer) = ensure_overlay_buffer(&mut guard.buffer, hwnd, cap_width, cap_height)
        else {
            EndPaint(hwnd, &ps);
            return;
        };
        let dirty = ps.rcPaint;
        let dirty_w = dirty.right - dirty.left;
        let dirty_h = dirty.bottom - dirty.top;
        if dirty_w <= 0 || dirty_h <= 0 {
            EndPaint(hwnd, &ps);
            return;
        }
        let dirty_rgn = CreateRectRgn(dirty.left, dirty.top, dirty.right, dirty.bottom);
        if dirty_rgn.0 != 0 {
            let _ = SelectClipRgn(buffer.dc, dirty_rgn);
        }
        let _ = windows::Win32::Graphics::Gdi::StretchDIBits(
            buffer.dc,
            0,
            0,
            cap_width,
            cap_height,
            0,
            0,
            cap_width,
            cap_height,
            Some(dimmed_ptr as *const _),
            &mut bmi,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
        if let Some(mut rect) = rect_opt {
            rect = clamp_rect_to_bounds(rect, cap_left, cap_top, cap_width, cap_height);
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            if width > 0 && height > 0 {
                let sx = rect.left - cap_left;
                let sy = rect.top - cap_top;
                let rgn = CreateRectRgn(sx, sy, sx + width, sy + height);
                if rgn.0 != 0 {
                    let _ = SelectClipRgn(buffer.dc, rgn);
                    let _ = windows::Win32::Graphics::Gdi::StretchDIBits(
                        buffer.dc,
                        0,
                        0,
                        cap_width,
                        cap_height,
                        0,
                        0,
                        cap_width,
                        cap_height,
                        Some(capture_ptr as *const _),
                        &mut bmi,
                        DIB_RGB_COLORS,
                        SRCCOPY,
                    );
                    if dirty_rgn.0 != 0 {
                        let _ = SelectClipRgn(buffer.dc, dirty_rgn);
                    }
                    DeleteObject(HGDIOBJ(rgn.0));
                }
            }
            if dirty_rgn.0 != 0 {
                let _ = SelectClipRgn(buffer.dc, dirty_rgn);
            }
            let pen = CreatePen(PS_SOLID, 2, rgb_color(255, 255, 255));
            let old_pen = SelectObject(buffer.dc, HGDIOBJ(pen.0));
            let null_brush = GetStockObject(NULL_BRUSH);
            let old_brush = SelectObject(buffer.dc, null_brush);
            windows::Win32::Graphics::Gdi::Rectangle(
                buffer.dc,
                rect.left - cap_left,
                rect.top - cap_top,
                rect.right - cap_left,
                rect.bottom - cap_top,
            );
            SelectObject(buffer.dc, old_brush);
            SelectObject(buffer.dc, old_pen);
            DeleteObject(HGDIOBJ(pen.0));
            let label = format!("{} x {}", width, height);
            SetBkMode(buffer.dc, TRANSPARENT);
            SetTextColor(buffer.dc, rgb_color(255, 255, 255));
            let pos = cursor;
            let wide: Vec<u16> = label.encode_utf16().collect();
            let _ = TextOutW(buffer.dc, pos.x + 12, pos.y + 12, &wide);
        }
        let _ = SelectClipRgn(buffer.dc, HRGN(0));
        if dirty_rgn.0 != 0 {
            DeleteObject(HGDIOBJ(dirty_rgn.0));
        }
        let _ = BitBlt(
            hdc, dirty.left, dirty.top, dirty_w, dirty_h, buffer.dc, dirty.left, dirty.top, SRCCOPY,
        );
        EndPaint(hwnd, &ps);
    }
}

unsafe extern "system" fn monitor_overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            set_cross_cursor();
            LRESULT(0)
        }
        WM_SETCURSOR => {
            set_cross_cursor();
            LRESULT(1)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_MOUSEMOVE => {
            let client = lparam_point(lparam);
            with_monitor_overlay_state(|s| s.cursor = client);
            let screen_pt = monitor_overlay_to_screen(client);
            if let Some((monitor, rect)) = monitor_rect_at_point(screen_pt) {
                with_monitor_overlay_state(|s| {
                    s.current_monitor = Some(monitor);
                    s.current_rect = Some(rect);
                });
            } else {
                with_monitor_overlay_state(|s| {
                    s.current_monitor = None;
                    s.current_rect = None;
                });
            }
            invalidate(hwnd);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let selection = with_monitor_overlay_state(|s| {
                let rect = s.current_rect;
                let monitor = s.current_monitor;
                if let (Some(monitor), Some(rect)) = (monitor, rect) {
                    s.result = Some(MonitorSelection {
                        monitor: Some(monitor),
                        rect,
                    });
                }
                s.done = true;
                s.result
            })
            .unwrap_or(None);
            if selection.is_none() {
                with_monitor_overlay_state(|s| s.canceled = true);
            }
            LRESULT(0)
        }
        WM_RBUTTONDOWN | WM_RBUTTONUP | WM_KEYDOWN => {
            if msg == WM_KEYDOWN && wparam.0 as u32 != 0x1B {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            with_monitor_overlay_state(|s| {
                s.canceled = true;
                s.done = true;
            });
            LRESULT(0)
        }
        WM_PAINT => {
            paint_monitor_overlay(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => LRESULT(0),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn run_monitor_overlay(capture: ScreenCapture) -> Result<Option<MonitorSelection>> {
    let state =
        MONITOR_OVERLAY_STATE.get_or_init(|| std::sync::Mutex::new(MonitorOverlayState::default()));
    {
        let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
        let dimmed = dim_pixels(&capture.pixels);
        *guard = MonitorOverlayState {
            capture: Some(capture),
            dimmed: Some(dimmed),
            buffer: None,
            current_rect: None,
            current_monitor: None,
            cursor: POINT::default(),
            done: false,
            canceled: false,
            result: None,
            overlay_hwnd: HWND(0),
        };
    }
    unsafe {
        let hinstance: HINSTANCE =
            windows::Win32::System::LibraryLoader::GetModuleHandleW(None)?.into();
        let class_name = w!("HyprshotMonitorOverlay");
        let wnd = WNDCLASSW {
            lpfnWndProc: Some(monitor_overlay_wnd_proc),
            hInstance: hinstance,
            lpszClassName: class_name,
            style: CS_HREDRAW | CS_VREDRAW,
            ..Default::default()
        };
        RegisterClassW(&wnd);
        let (left, top, width, height) = with_monitor_overlay_state(|s| {
            let c = s.capture.as_ref().unwrap();
            (c.left, c.top, c.width, c.height)
        })
        .unwrap_or((0, 0, 0, 0));
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name,
            w!(""),
            WS_POPUP | WS_VISIBLE,
            left,
            top,
            width,
            height,
            HWND(0),
            HMENU(0),
            hinstance,
            None,
        );
        if hwnd.0 == 0 {
            return Ok(None);
        }
        with_monitor_overlay_state(|s| s.overlay_hwnd = hwnd);
        SetForegroundWindow(hwnd);
        let mut msg = MSG::default();
        loop {
            while PeekMessageW(&mut msg, HWND(0), 0, 0, PM_REMOVE).as_bool() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let done = with_monitor_overlay_state(|s| s.done).unwrap_or(true);
            if done {
                break;
            }
            if escape_pressed() {
                with_monitor_overlay_state(|s| {
                    s.canceled = true;
                    s.done = true;
                });
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let result = with_monitor_overlay_state(|s| {
            let output = if s.canceled { None } else { s.result };
            s.capture = None;
            destroy_overlay_buffer(&mut s.buffer);
            output
        })
        .unwrap_or(None);
        let _ = DestroyWindow(hwnd);
        Ok(result)
    }
}

fn is_valid_capture_window(hwnd: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return false;
        }
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if (ex & WS_EX_TOOLWINDOW.0 as u32) != 0 || (ex & WS_EX_NOACTIVATE.0 as u32) != 0 {
            return false;
        }
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return false;
        }
        (rect.right - rect.left) > 4 && (rect.bottom - rect.top) > 4
    }
}

fn window_rect(hwnd: HWND) -> Option<CaptureRect> {
    unsafe {
        let mut rect = RECT::default();
        let ok = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rect as *mut _ as *mut _,
            size_of::<RECT>() as u32,
        )
        .is_ok();
        if !ok {
            if GetWindowRect(hwnd, &mut rect).is_err() {
                return None;
            }
        }
        if rect.right <= rect.left || rect.bottom <= rect.top {
            return None;
        }
        Some(CaptureRect {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        })
    }
}

pub(crate) fn clamp_rect_to_bounds(
    mut rect: CaptureRect,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
) -> CaptureRect {
    rect.left = rect.left.clamp(left, left + width);
    rect.right = rect.right.clamp(left, left + width);
    rect.top = rect.top.clamp(top, top + height);
    rect.bottom = rect.bottom.clamp(top, top + height);
    rect
}

pub(crate) fn point_in_rect(pt: POINT, rect: CaptureRect) -> bool {
    pt.x >= rect.left && pt.x < rect.right && pt.y >= rect.top && pt.y < rect.bottom
}

pub(crate) fn crop_to_rgba(
    capture: &ScreenCapture,
    rect: &CaptureRect,
) -> Result<(Vec<u8>, u32, u32)> {
    let left = rect.left.max(capture.left);
    let top = rect.top.max(capture.top);
    let right = rect.right.min(capture.left + capture.width);
    let bottom = rect.bottom.min(capture.top + capture.height);
    let width = (right - left).max(0);
    let height = (bottom - top).max(0);
    if width <= 0 || height <= 0 {
        anyhow::bail!("Invalid selection size");
    }
    let x = left - capture.left;
    let y = top - capture.top;
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for row in 0..height {
        let src_offset = ((y + row) * capture.width + x) * 4;
        let dst_offset = (row * width) * 4;
        let src = &capture.pixels[src_offset as usize..(src_offset + width * 4) as usize];
        let dst = &mut rgba[dst_offset as usize..(dst_offset + width * 4) as usize];
        for px in 0..width {
            let si = (px * 4) as usize;
            dst[si] = src[si + 2];
            dst[si + 1] = src[si + 1];
            dst[si + 2] = src[si];
            dst[si + 3] = src[si + 3];
        }
    }
    Ok((rgba, width as u32, height as u32))
}

pub(crate) fn save_png(path: &Path, rgba: &[u8], width: u32, height: u32) -> Result<()> {
    let img = image::RgbaImage::from_raw(width, height, rgba.to_vec())
        .context("Failed to create image")?;
    img.save(path).context("Failed to save PNG")?;
    Ok(())
}

fn copy_to_clipboard(rgba: &[u8], width: u32, height: u32) -> Result<()> {
    let mut clipboard = Clipboard::new().context("Clipboard unavailable")?;
    clipboard
        .set_image(ImageData {
            width: width as usize,
            height: height as usize,
            bytes: Cow::Owned(rgba.to_vec()),
        })
        .context("Failed to copy image")?;
    Ok(())
}

fn default_filename() -> String {
    let now = chrono::Local::now();
    format!(
        "{}-{:03}_hyprshot.png",
        now.format("%Y-%m-%d-%H%M%S"),
        now.timestamp_subsec_millis()
    )
}
