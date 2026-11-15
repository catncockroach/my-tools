//! Exe tự cài đặt / cập nhật / gỡ, không cần installer riêng hay server.
//!
//! - Chạy exe từ chỗ khác (vd. Downloads): chép vào `%LOCALAPPDATA%\Programs\screen-hud`,
//!   đăng ký Settings > Apps, bật khởi động cùng Windows (lần cài đầu), rồi chạy bản đã cài.
//!   Nếu đang có bản chạy thì tắt nó trước: đây chính là cách cập nhật (tải bản mới, chạy).
//! - `--uninstall` (Settings > Apps gọi): tắt bản đang chạy, xoá registry, xoá thư mục cài.
//! - Mọi thứ nằm trong HKCU và thư mục của user: không cần quyền admin.

use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Registry::*;
use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::wide;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const RELEASES_URL: &str = "https://github.com/catncockroach/my-tools/releases";
/// Tên class cửa sổ điều khiển; bản cài mới dùng nó để tìm và tắt bản đang chạy.
pub const CTL_CLASS: &str = "ScreenHudCtl";

const APP: &str = "screen-hud";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\screen-hud";
const SETTINGS_KEY: &str = r"Software\screen-hud";
const THEME_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";

pub fn install_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default()).join("Programs").join(APP)
}

pub fn installed_exe() -> PathBuf {
    install_dir().join("screen-hud.exe")
}

fn current_exe() -> PathBuf {
    std::env::current_exe().unwrap_or_default()
}

/// Process này có phải bản đã cài không.
pub fn is_installed_copy() -> bool {
    current_exe().to_string_lossy().to_lowercase() == installed_exe().to_string_lossy().to_lowercase()
}

/// Cài hoặc cập nhật bản đang chạy vào thư mục cài. Trả về true nếu là lần cài đầu.
pub fn install() -> Result<bool, String> {
    let dst = installed_exe();
    let fresh = !dst.exists();
    stop_running();
    std::fs::create_dir_all(install_dir()).map_err(|e| format!("Không tạo được {}: {e}", install_dir().display()))?;
    // Bản cũ vừa thoát có thể còn giữ file thêm một chút.
    let src = current_exe();
    let mut attempt = 0;
    loop {
        match std::fs::copy(&src, &dst) {
            Ok(_) => break,
            Err(_) if attempt < 15 => {
                attempt += 1;
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            Err(e) => return Err(format!("Không chép được vào {}: {e}", dst.display())),
        }
    }
    if fresh || autostart() {
        set_autostart_path(Some(&dst));
    }
    let exe = dst.to_string_lossy().to_string();
    let size_kb = std::fs::metadata(&dst).map(|m| (m.len() / 1024) as u32).unwrap_or(0);
    set_str(UNINSTALL_KEY, "DisplayName", APP);
    set_str(UNINSTALL_KEY, "DisplayVersion", VERSION);
    set_str(UNINSTALL_KEY, "Publisher", "catncockroach");
    set_str(UNINSTALL_KEY, "DisplayIcon", &exe);
    set_str(UNINSTALL_KEY, "InstallLocation", &install_dir().to_string_lossy());
    set_str(UNINSTALL_KEY, "UninstallString", &format!("\"{exe}\" --uninstall"));
    set_str(UNINSTALL_KEY, "URLInfoAbout", RELEASES_URL);
    set_dword(UNINSTALL_KEY, "NoModify", 1);
    set_dword(UNINSTALL_KEY, "NoRepair", 1);
    set_dword(UNINSTALL_KEY, "EstimatedSize", size_kb);
    Ok(fresh)
}

/// Chạy bản đã cài (sau khi cài/cập nhật).
pub fn launch_installed() -> Result<(), String> {
    std::process::Command::new(installed_exe())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Không chạy được {}: {e}", installed_exe().display()))
}

/// Gỡ phần registry và tắt bản đang chạy. Gọi [`remove_files`] ngay trước khi process thoát.
pub fn uninstall() {
    stop_running();
    set_autostart_path(None);
    delete_tree(UNINSTALL_KEY);
    delete_tree(SETTINGS_KEY);
}

/// Xoá thư mục cài. Exe đang chạy không tự xoá được, nên nhờ cmd xoá sau khi process này
/// thoát (chờ ~2 s); vì vậy chỉ gọi khi sắp thoát, không còn hộp thoại nào đang mở.
pub fn remove_files() {
    let dir = install_dir();
    if is_installed_copy() {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = std::process::Command::new("cmd.exe")
            .raw_arg(format!("/s /c \"ping -n 3 127.0.0.1 >nul & rmdir /s /q \"{}\"\"", dir.display()))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    } else {
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// Tắt bản screen-hud đang chạy (nếu có) và chờ nó thoát, tối đa 5 s.
pub fn stop_running() {
    unsafe {
        let class = wide(CTL_CLASS);
        let hwnd = FindWindowW(class.as_ptr(), null());
        if hwnd.is_null() {
            return;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let process = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        PostMessageW(hwnd, WM_CLOSE, 0, 0);
        if process.is_null() {
            std::thread::sleep(std::time::Duration::from_secs(1));
        } else {
            WaitForSingleObject(process, 5000);
            CloseHandle(process);
        }
    }
}

pub fn autostart() -> bool {
    let k = wide(RUN_KEY);
    let n = wide(APP);
    unsafe { RegGetValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ptr(), RRF_RT_REG_SZ, null_mut(), null_mut(), null_mut()) == ERROR_SUCCESS }
}

/// Bật khởi động cùng Windows cho exe đang chạy.
pub fn set_autostart(on: bool) {
    set_autostart_path(on.then(current_exe).as_deref());
}

fn set_autostart_path(exe: Option<&std::path::Path>) {
    match exe {
        Some(p) => set_str(RUN_KEY, APP, &format!("\"{}\"", p.display())),
        None => {
            let k = wide(RUN_KEY);
            let n = wide(APP);
            unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ptr()) };
        }
    }
}

pub fn show_overlay() -> bool {
    get_dword(SETTINGS_KEY, "ShowOverlay").map(|v| v != 0).unwrap_or(true)
}

pub fn set_show_overlay(on: bool) {
    set_dword(SETTINGS_KEY, "ShowOverlay", on as u32);
}

/// Taskbar đang dùng theme sáng (icon khay cần màu tối).
pub fn light_taskbar() -> bool {
    get_dword(THEME_KEY, "SystemUsesLightTheme") == Some(1)
}

fn set_str(key: &str, name: &str, val: &str) {
    let (k, n, v) = (wide(key), wide(name), wide(val));
    unsafe {
        RegSetKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ptr(), REG_SZ, v.as_ptr() as _, (v.len() * 2) as u32);
    }
}

fn set_dword(key: &str, name: &str, val: u32) {
    let (k, n) = (wide(key), wide(name));
    unsafe {
        RegSetKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), n.as_ptr(), REG_DWORD, &val as *const u32 as _, 4);
    }
}

fn get_dword(key: &str, name: &str) -> Option<u32> {
    let (k, n) = (wide(key), wide(name));
    let mut v = 0u32;
    let mut size = 4u32;
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            k.as_ptr(),
            n.as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            &mut v as *mut u32 as _,
            &mut size,
        )
    };
    (r == ERROR_SUCCESS).then_some(v)
}

fn delete_tree(key: &str) {
    let k = wide(key);
    unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, k.as_ptr()) };
}
