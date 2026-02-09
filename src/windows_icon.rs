#![allow(unsafe_op_in_unsafe_fn)]

use std::sync::OnceLock;

use windows::Win32::Foundation::{BOOL, HWND};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS,
    DeleteObject, GetDC, HBITMAP, HGDIOBJ, ReleaseDC,
};
use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, HICON, ICONINFO};

const VIEWBOX_SIZE: f32 = 24.0;
const TRAY_ICON_SIZE: u32 = 32;
const UI_LOGO_SIZE: u32 = 128;

static TRAY_ICON_HANDLE: OnceLock<HICON> = OnceLock::new();
static UI_LOGO_LIGHT_RGBA: OnceLock<Vec<u8>> = OnceLock::new();
static UI_LOGO_DARK_RGBA: OnceLock<Vec<u8>> = OnceLock::new();

pub fn tray_icon() -> Option<HICON> {
    let icon = TRAY_ICON_HANDLE.get_or_init(|| {
        let rgba = render_logo(TRAY_ICON_SIZE, [25, 25, 25, 255]);
        rgba_to_hicon(&rgba, TRAY_ICON_SIZE, TRAY_ICON_SIZE).unwrap_or(HICON(0))
    });
    if icon.0 == 0 { None } else { Some(*icon) }
}

pub fn ui_logo_rgba(dark: bool) -> Option<(Vec<u8>, u32, u32)> {
    let rgba = if dark {
        UI_LOGO_DARK_RGBA
            .get_or_init(|| render_logo(UI_LOGO_SIZE, [230, 230, 230, 255]))
            .clone()
    } else {
        UI_LOGO_LIGHT_RGBA
            .get_or_init(|| render_logo(UI_LOGO_SIZE, [25, 25, 25, 255]))
            .clone()
    };
    Some((rgba, UI_LOGO_SIZE, UI_LOGO_SIZE))
}

fn render_logo(size: u32, color: [u8; 4]) -> Vec<u8> {
    let mut buf = vec![0u8; (size * size * 4) as usize];
    let scale = size as f32 / VIEWBOX_SIZE;
    let stroke = (2.0 * scale).max(1.0);

    draw_vline(&mut buf, size, 4.0, 8.0, 4.0, stroke, color);
    draw_hline(&mut buf, size, 4.0, 8.0, 4.0, stroke, color);

    draw_hline(&mut buf, size, 16.0, 20.0, 4.0, stroke, color);
    draw_vline(&mut buf, size, 20.0, 4.0, 8.0, stroke, color);

    draw_vline(&mut buf, size, 20.0, 16.0, 20.0, stroke, color);
    draw_hline(&mut buf, size, 20.0, 16.0, 20.0, stroke, color);

    draw_hline(&mut buf, size, 8.0, 4.0, 20.0, stroke, color);
    draw_vline(&mut buf, size, 4.0, 20.0, 16.0, stroke, color);

    draw_hline(&mut buf, size, 9.0, 15.0, 12.0, stroke, color);
    draw_vline(&mut buf, size, 12.0, 9.0, 15.0, stroke, color);

    buf
}

fn draw_hline(buf: &mut [u8], size: u32, x1: f32, x2: f32, y: f32, stroke: f32, color: [u8; 4]) {
    let scale = size as f32 / VIEWBOX_SIZE;
    let (x_start, x_end) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
    let x0 = x_start * scale - stroke / 2.0;
    let x1 = x_end * scale + stroke / 2.0;
    let y0 = y * scale - stroke / 2.0;
    let y1 = y * scale + stroke / 2.0;
    fill_rect(buf, size, x0, y0, x1, y1, color);
}

fn draw_vline(buf: &mut [u8], size: u32, x: f32, y1: f32, y2: f32, stroke: f32, color: [u8; 4]) {
    let scale = size as f32 / VIEWBOX_SIZE;
    let (y_start, y_end) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
    let x0 = x * scale - stroke / 2.0;
    let x1 = x * scale + stroke / 2.0;
    let y0 = y_start * scale - stroke / 2.0;
    let y1 = y_end * scale + stroke / 2.0;
    fill_rect(buf, size, x0, y0, x1, y1, color);
}

fn fill_rect(buf: &mut [u8], size: u32, x0: f32, y0: f32, x1: f32, y1: f32, color: [u8; 4]) {
    let size_i = size as i32;
    let mut left = x0.floor() as i32;
    let mut right = x1.ceil() as i32;
    let mut top = y0.floor() as i32;
    let mut bottom = y1.ceil() as i32;
    left = left.clamp(0, size_i);
    right = right.clamp(0, size_i);
    top = top.clamp(0, size_i);
    bottom = bottom.clamp(0, size_i);
    if right <= left || bottom <= top {
        return;
    }
    for y in top..bottom {
        for x in left..right {
            let idx = ((y as u32 * size + x as u32) * 4) as usize;
            buf[idx] = color[0];
            buf[idx + 1] = color[1];
            buf[idx + 2] = color[2];
            buf[idx + 3] = color[3];
        }
    }
}

fn rgba_to_hicon(rgba: &[u8], width: u32, height: u32) -> Option<HICON> {
    unsafe {
        let mut bits_ptr = std::ptr::null_mut();
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let hdc = GetDC(HWND(0));
        let dib = CreateDIBSection(hdc, &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0).ok()?;
        ReleaseDC(HWND(0), hdc);
        if dib.0 == 0 || bits_ptr.is_null() {
            if dib.0 != 0 {
                DeleteObject(HGDIOBJ(dib.0));
            }
            return None;
        }
        let dst =
            std::slice::from_raw_parts_mut(bits_ptr as *mut u8, (width * height * 4) as usize);
        for i in 0..(width * height) as usize {
            let si = i * 4;
            dst[si] = rgba[si + 2];
            dst[si + 1] = rgba[si + 1];
            dst[si + 2] = rgba[si];
            dst[si + 3] = rgba[si + 3];
        }
        let mask = CreateBitmap(width as i32, height as i32, 1, 1, None);
        let icon_info = ICONINFO {
            fIcon: BOOL(1),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: HBITMAP(mask.0),
            hbmColor: dib,
        };
        let icon = match CreateIconIndirect(&icon_info) {
            Ok(icon) => icon,
            Err(_) => {
                return None;
            }
        };
        DeleteObject(HGDIOBJ(mask.0));
        DeleteObject(HGDIOBJ(dib.0));

        if icon.0 == 0 { None } else { Some(icon) }
    }
}
