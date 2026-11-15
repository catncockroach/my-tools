//! Icon khay hệ thống + menu chuột phải.
//!
//! Icon được vẽ lúc chạy từ font icon của Windows (không cần file .ico/resource): một ký tự
//! pin theo mức pin thấp nhất, hoặc ký tự Bluetooth khi không có thiết bị nào kết nối.

use std::ffi::c_void;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{HWND, POINT, SIZE};
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::HiDpi::{GetDpiForSystem, GetSystemMetricsForDpi};
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const ID: u32 = 1;

/// Ký tự Bluetooth trong Segoe Fluent Icons / Segoe MDL2 Assets.
pub const GLYPH_BLUETOOTH: u16 = 0xE702;

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

/// Vẽ icon khay từ một ký tự font icon; `dark` = màu đen (cho taskbar sáng).
/// Trả về null nếu thất bại (khi đó dùng icon mặc định).
pub unsafe fn make_icon(glyph: u16, dark: bool) -> HICON {
    let size = GetSystemMetricsForDpi(SM_CXSMICON, GetDpiForSystem()).max(16);
    let screen = GetDC(null_mut());
    let mem = CreateCompatibleDC(screen);
    let font = crate::overlay::make_icon_font(mem, size);

    let mut bmi: BITMAPINFO = std::mem::zeroed();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = size;
    bmi.bmiHeader.biHeight = -size;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;
    let mut bits: *mut c_void = null_mut();
    let dib = CreateDIBSection(mem, &bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
    let mut icon: HICON = null_mut();
    if !dib.is_null() && !bits.is_null() {
        let old_bmp = SelectObject(mem, dib);
        let old_font = SelectObject(mem, font);
        let px = std::slice::from_raw_parts_mut(bits as *mut u32, (size * size) as usize);
        px.fill(0);
        SetBkMode(mem, TRANSPARENT as i32);
        SetTextColor(mem, 0x00FF_FFFF);
        let mut ext = SIZE { cx: 0, cy: 0 };
        GetTextExtentPoint32W(mem, &glyph, 1, &mut ext);
        TextOutW(mem, (size - ext.cx) / 2, (size - ext.cy) / 2, &glyph, 1);
        GdiFlush();
        let c: u32 = if dark { 0x00 } else { 0xFF };
        for p in px.iter_mut() {
            let a = ((*p >> 16) & 0xFF).max((*p >> 8) & 0xFF).max(*p & 0xFF);
            *p = a << 24 | c << 16 | c << 8 | c;
        }
        SelectObject(mem, old_font);
        SelectObject(mem, old_bmp);

        let mask = CreateBitmap(size, size, 1, 1, null());
        let info = ICONINFO { fIcon: 1, xHotspot: 0, yHotspot: 0, hbmMask: mask, hbmColor: dib };
        icon = CreateIconIndirect(&info);
        DeleteObject(mask);
    }
    if !dib.is_null() {
        DeleteObject(dib);
    }
    DeleteObject(font);
    DeleteDC(mem);
    ReleaseDC(null_mut(), screen);
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
