//! Icon khay hệ thống + menu chuột phải.

use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{HWND, POINT};
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const ID: u32 = 1;

unsafe fn data(hwnd: HWND, callback: u32, tip: &str) -> NOTIFYICONDATAW {
    let mut d: NOTIFYICONDATAW = std::mem::zeroed();
    d.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    d.hWnd = hwnd;
    d.uID = ID;
    d.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    d.uCallbackMessage = callback;
    d.hIcon = LoadIconW(null_mut(), IDI_APPLICATION);
    for (dst, src) in d.szTip.iter_mut().take(127).zip(tip.encode_utf16()) {
        *dst = src;
    }
    d
}

pub unsafe fn add(hwnd: HWND, callback: u32, tip: &str) {
    let d = data(hwnd, callback, tip);
    Shell_NotifyIconW(NIM_ADD, &d);
}

pub unsafe fn set_tip(hwnd: HWND, callback: u32, tip: &str) {
    let d = data(hwnd, callback, tip);
    Shell_NotifyIconW(NIM_MODIFY, &d);
}

pub unsafe fn remove(hwnd: HWND) {
    let mut d: NOTIFYICONDATAW = std::mem::zeroed();
    d.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    d.hWnd = hwnd;
    d.uID = ID;
    Shell_NotifyIconW(NIM_DELETE, &d);
}

pub enum Item<'a> {
    Check(usize, &'a str, bool),
    Action(usize, &'a str),
    Separator,
}

/// Hiện menu tại con trỏ chuột, trả về id mục được chọn (0 nếu bấm ra ngoài).
pub unsafe fn menu(hwnd: HWND, items: &[Item]) -> usize {
    let m = CreatePopupMenu();
    for it in items {
        match it {
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
