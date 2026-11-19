//! Icon khay hệ thống + menu chuột phải.
//!
//! Icon (con vịt) được vẽ lúc chạy (không cần file .ico/resource): ghép vài hình elip,
//! lấy mẫu 4x4 mỗi pixel để khử răng cưa.

use std::ffi::c_void;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{HWND, POINT};
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::HiDpi::{GetDpiForSystem, GetSystemMetricsForDpi};
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const ID: u32 = 1;

/// Thêm (`add`) hoặc cập nhật icon khay.
pub unsafe fn set(hwnd: HWND, callback: u32, tip: &str, icon: HICON, add: bool) {
    let mut d: NOTIFYICONDATAW = std::mem::zeroed();
    d.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    d.hWnd = hwnd;
    d.uID = ID;
    d.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    d.uCallbackMessage = callback;
    d.hIcon = if icon.is_null() { LoadIconW(null_mut(), IDI_APPLICATION) } else { icon };
    for (dst, src) in d.szTip.iter_mut().take(127).zip(tip.encode_utf16()) {
        *dst = src;
    }
    Shell_NotifyIconW(if add { NIM_ADD } else { NIM_MODIFY }, &d);
}

pub unsafe fn remove(hwnd: HWND) {
    let mut d: NOTIFYICONDATAW = std::mem::zeroed();
    d.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    d.hWnd = hwnd;
    d.uID = ID;
    Shell_NotifyIconW(NIM_DELETE, &d);
}

/// Một hình elip (có thể xoay) của con vịt, toạ độ trên lưới 16x16, y hướng xuống.
struct Blob {
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    /// Góc xoay (độ), dương = theo chiều kim đồng hồ trên màn hình.
    deg: f32,
    color: [u8; 3],
}

const YELLOW: [u8; 3] = [255, 208, 56];
/// Thứ tự vẽ: hình sau đè hình trước.
const DUCK: [Blob; 6] = [
    Blob { cx: 13.6, cy: 8.6, rx: 2.3, ry: 1.15, deg: -35.0, color: YELLOW }, // đuôi
    Blob { cx: 9.0, cy: 11.2, rx: 6.2, ry: 3.9, deg: 0.0, color: YELLOW },    // thân
    Blob { cx: 9.9, cy: 10.6, rx: 3.0, ry: 1.6, deg: -12.0, color: [236, 170, 20] }, // cánh
    Blob { cx: 5.8, cy: 5.4, rx: 3.4, ry: 3.4, deg: 0.0, color: YELLOW },     // đầu
    Blob { cx: 2.3, cy: 6.7, rx: 1.7, ry: 0.95, deg: 8.0, color: [255, 128, 32] }, // mỏ
    Blob { cx: 5.1, cy: 4.6, rx: 0.85, ry: 0.85, deg: 0.0, color: [30, 30, 30] }, // mắt
];
/// Viền tối quanh con vịt để nhìn rõ trên taskbar sáng.
const OUTLINE: f32 = 0.7;
const OUTLINE_COLOR: [u8; 3] = [92, 60, 0];

fn inside(b: &Blob, x: f32, y: f32, grow: f32) -> bool {
    let (sin, cos) = b.deg.to_radians().sin_cos();
    let (dx, dy) = (x - b.cx, y - b.cy);
    let lx = (dx * cos + dy * sin) / (b.rx + grow);
    let ly = (-dx * sin + dy * cos) / (b.ry + grow);
    lx * lx + ly * ly <= 1.0
}

fn duck_color(x: f32, y: f32) -> Option<[u8; 3]> {
    if let Some(b) = DUCK.iter().rev().find(|b| inside(b, x, y, 0.0)) {
        return Some(b.color);
    }
    DUCK[..4].iter().any(|b| inside(b, x, y, OUTLINE)).then_some(OUTLINE_COLOR)
}

/// Vẽ icon khay con vịt. Trả về null nếu thất bại (khi đó dùng icon mặc định).
pub unsafe fn make_icon() -> HICON {
    let size = GetSystemMetricsForDpi(SM_CXSMICON, GetDpiForSystem()).max(16);
    let mut bmi: BITMAPINFO = std::mem::zeroed();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = size;
    bmi.bmiHeader.biHeight = -size;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;
    let mut bits: *mut c_void = null_mut();
    let dib = CreateDIBSection(null_mut(), &bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
    if dib.is_null() || bits.is_null() {
        if !dib.is_null() {
            DeleteObject(dib);
        }
        return null_mut();
    }
    let px = std::slice::from_raw_parts_mut(bits as *mut u32, (size * size) as usize);
    const N: usize = 4;
    let unit = 16.0 / size as f32;
    for y in 0..size as usize {
        for x in 0..size as usize {
            let (mut sum, mut hit) = ([0u32; 3], 0u32);
            for sy in 0..N {
                for sx in 0..N {
                    let u = (x as f32 + (sx as f32 + 0.5) / N as f32) * unit;
                    let v = (y as f32 + (sy as f32 + 0.5) / N as f32) * unit;
                    if let Some(c) = duck_color(u, v) {
                        for i in 0..3 {
                            sum[i] += c[i] as u32;
                        }
                        hit += 1;
                    }
                }
            }
            // Alpha thẳng (không premultiplied), như trong file .ico.
            px[y * size as usize + x] = if hit == 0 {
                0
            } else {
                let a = hit * 255 / (N * N) as u32;
                a << 24 | (sum[0] / hit) << 16 | (sum[1] / hit) << 8 | (sum[2] / hit)
            };
        }
    }
    GdiFlush();
    let mask = CreateBitmap(size, size, 1, 1, null());
    let info = ICONINFO { fIcon: 1, xHotspot: 0, yHotspot: 0, hbmMask: mask, hbmColor: dib };
    let icon = CreateIconIndirect(&info);
    DeleteObject(mask);
    DeleteObject(dib);
    icon
}

pub enum Item<'a> {
    Label(&'a str),
    Check(usize, &'a str, bool),
    Action(usize, &'a str),
    Separator,
}

/// Hiện menu tại con trỏ chuột, trả về id mục được chọn (0 nếu bấm ra ngoài).
pub unsafe fn menu(hwnd: HWND, items: &[Item]) -> usize {
    let m = CreatePopupMenu();
    for it in items {
        match it {
            Item::Label(text) => {
                let t = crate::wide(text);
                AppendMenuW(m, MF_STRING | MF_GRAYED, 0, t.as_ptr());
            }
            Item::Check(id, text, on) => {
                let t = crate::wide(text);
                AppendMenuW(m, MF_STRING | if *on { MF_CHECKED } else { MF_UNCHECKED }, *id, t.as_ptr());
            }
            Item::Action(id, text) => {
                let t = crate::wide(text);
                AppendMenuW(m, MF_STRING, *id, t.as_ptr());
            }
            Item::Separator => {
                AppendMenuW(m, MF_SEPARATOR, 0, null());
            }
        }
    }
    let mut pt = POINT { x: 0, y: 0 };
    GetCursorPos(&mut pt);
    // Bắt buộc để menu tự đóng khi bấm ra ngoài.
    SetForegroundWindow(hwnd);
    let chosen = TrackPopupMenu(m, TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY, pt.x, pt.y, 0, hwnd, null());
    PostMessageW(hwnd, WM_NULL, 0, 0);
    DestroyMenu(m);
    chosen.max(0) as usize
}
