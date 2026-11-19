//! screen-hud: process nền hiển thị overlay mờ ở góc dưới (phải hoặc trái) màn hình.
//!
//! Thiết kế để tốn ít tài nguyên nhất có thể:
//! - không có vòng lặp vẽ: chỉ vẽ lại khi dữ liệu đổi;
//! - DevQuery báo khi thiết bị đổi trạng thái, cộng một lần đọc lại mỗi 60 s cho % pin;
//! - sau mỗi lần cập nhật, trả lại bộ nhớ working set không dùng cho hệ thống.
//!
//! Dòng lệnh:
//!   screen-hud.exe             chạy từ ngoài thư mục cài: cài/cập nhật rồi chạy bản đã cài
//!                              (xem install.rs); chạy từ thư mục cài: chạy nền
//!   screen-hud.exe --portable  chạy nền tại chỗ, không cài (dùng khi phát triển)
//!   screen-hud.exe --uninstall gỡ (Settings > Apps gọi)
//!   screen-hud.exe --all       hiện cả thiết bị chưa kết nối (chỉ để chẩn đoán)
//!   screen-hud.exe --list      in danh sách thiết bị ra stdout rồi thoát
//!   screen-hud.exe --log       ghi nhật ký sự kiện vào %TEMP%\screen-hud.log (chẩn đoán)

#![windows_subsystem = "windows"]

mod battery;
mod install;
mod overlay;
mod tray;

use std::cell::RefCell;
use std::io::Write;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Devices::DeviceQuery::HDEVQUERY;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{CreateMutexW, GetCurrentProcess, SetProcessWorkingSetSize};
use windows_sys::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use battery::Device;
use overlay::Line;

const WM_APP_DEVICES: u32 = WM_APP + 1;
const WM_APP_TRAY: u32 = WM_APP + 2;
const WM_APP_REDRAW: u32 = WM_APP + 3;

const TIMER_DEBOUNCE: usize = 1;
const TIMER_POLL: usize = 2;
const TIMER_FULLSCREEN: usize = 3;

const MENU_SHOW: usize = 100;
const MENU_AUTOSTART: usize = 101;
const MENU_RELEASES: usize = 102;
const MENU_LEFT: usize = 103;
const MENU_RIGHT: usize = 104;
const MENU_EXIT: usize = 199;

const LOW_BATTERY: u8 = 20;

struct State {
    ctl: HWND,
    overlay: HWND,
    devices: Vec<Device>,
    show: bool,
    /// Overlay ở góc dưới trái thay vì dưới phải.
    left: bool,
    show_all: bool,
    fullscreen: bool,
    /// Nội dung đang hiển thị; None = overlay đang ẩn.
    drawn: Option<Vec<Line>>,
    taskbar_created: u32,
    queries: Vec<HDEVQUERY>,
    icon: HICON,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Mượn state trong thời gian ngắn. Không gọi API có thể gửi message ngược vào wndproc bên trong `f`.
fn with<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|s| s.try_borrow_mut().ok().and_then(|mut g| g.as_mut().map(f)))
}

static LOG: std::sync::OnceLock<std::sync::Mutex<std::fs::File>> = std::sync::OnceLock::new();
static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// Ghi một dòng nhật ký nếu chạy với `--log`; không làm gì nếu không bật.
macro_rules! log {
    ($($arg:tt)*) => {
        if let Some(f) = LOG.get() {
            if let Ok(mut f) = f.lock() {
                let t = START.get().map(|s| s.elapsed().as_secs_f32()).unwrap_or(0.0);
                let _ = writeln!(f, "[{:9.3}] {}", t, format_args!($($arg)*));
            }
        }
    };
}

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--list") {
        let mut out = std::io::stdout();
        let (devices, t_dev, t_con) = battery::snapshot_timed();
        for d in devices {
            let _ = writeln!(out, "{}\t{}%\t{}", d.name, d.level, if d.connected { "connected" } else { "disconnected" });
        }
        let _ = writeln!(out, "(devnode query {} ms, container query {} ms)", t_dev.as_millis(), t_con.as_millis());
        return;
    }
    let has = |flag: &str| args.iter().any(|a| a == flag);
    if has("--uninstall") {
        install::uninstall();
        message("screen-hud has been uninstalled.", MB_ICONINFORMATION);
        install::remove_files();
        return;
    }
    if !has("--portable") && !install::is_installed_copy() {
        match install::install().and_then(|fresh| install::launch_installed().map(|_| fresh)) {
            Ok(true) => message(
                &format!(
                    "screen-hud v{} is installed.\n\nIt runs in the system tray and starts with Windows.\nUpdate: download the new version and run it.\nUninstall: Settings > Apps.",
                    install::VERSION
                ),
                MB_ICONINFORMATION,
            ),
            Ok(false) => message(&format!("screen-hud has been updated to v{}.", install::VERSION), MB_ICONINFORMATION),
            Err(e) => message(&format!("screen-hud installation failed:\n{e}"), MB_ICONERROR),
        }
        return;
    }
    if has("--log") {
        let path = std::env::temp_dir().join("screen-hud.log");
        if let Ok(f) = std::fs::File::create(&path) {
            let _ = START.set(std::time::Instant::now());
            let _ = LOG.set(std::sync::Mutex::new(f));
        }
    }
    log!("start args={:?}", args);
    unsafe { run(has("--all")) }
}

fn message(text: &str, icon: MESSAGEBOX_STYLE) {
    let (t, c) = (wide(text), wide("screen-hud"));
    unsafe { MessageBoxW(null_mut(), t.as_ptr(), c.as_ptr(), MB_OK | MB_SETFOREGROUND | icon) };
}

unsafe fn run(show_all: bool) {
    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

    let mutex_name = wide("Local\\screen-hud");
    let _mutex = CreateMutexW(null(), 0, mutex_name.as_ptr());
    if GetLastError() == ERROR_ALREADY_EXISTS {
        log!("another instance holds the mutex, exiting");
        return;
    }

    let hinst = GetModuleHandleW(null());
    let ctl_class = wide(install::CTL_CLASS);
    let overlay_class = wide("ScreenHudOverlay");
    let title = wide("screen-hud");
    type Proc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;
    for (name, proc_) in [
        (&ctl_class, ctl_proc as Proc),
        (&overlay_class, overlay_proc as Proc),
    ] {
        let mut wc: WNDCLASSEXW = std::mem::zeroed();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(proc_);
        wc.hInstance = hinst;
        wc.lpszClassName = name.as_ptr();
        RegisterClassExW(&wc);
    }

    // Cửa sổ điều khiển: ẩn, nhận message của tray/timer/DevQuery.
    let ctl = CreateWindowExW(0, ctl_class.as_ptr(), title.as_ptr(), WS_POPUP, 0, 0, 0, 0, null_mut(), null_mut(), hinst, null());
    let overlay = CreateWindowExW(
        overlay::EX_STYLE,
        overlay_class.as_ptr(),
        title.as_ptr(),
        WS_POPUP,
        0,
        0,
        1,
        1,
        null_mut(),
        null_mut(),
        hinst,
        null(),
    );
    CTL.store(ctl as isize, std::sync::atomic::Ordering::Relaxed);
    log!("windows created ctl={:?} overlay={:?}", ctl, overlay);

    let taskbar_created = RegisterWindowMessageW(wide("TaskbarCreated").as_ptr());
    let icon = tray::make_icon();
    STATE.with(|s| {
        *s.borrow_mut() = Some(State {
            ctl,
            overlay,
            devices: Vec::new(),
            show: install::show_overlay(),
            left: install::overlay_left(),
            show_all,
            fullscreen: false,
            drawn: None,
            taskbar_created,
            queries: Vec::new(),
            icon,
        })
    });

    tray::set(ctl, WM_APP_TRAY, "screen-hud", icon, true);
    let queries = battery::watch(ctl, WM_APP_DEVICES);
    log!("device watch queries registered: {}", queries.len());
    with(|s| s.queries = queries);
    refresh();
    SetTimer(ctl, TIMER_POLL, 60_000, None);
    SetTimer(ctl, TIMER_FULLSCREEN, 2_000, None);

    let mut msg: MSG = std::mem::zeroed();
    while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    if let Some(q) = with(|s| std::mem::take(&mut s.queries)) {
        battery::unwatch(&q);
    }
    tray::remove(ctl);
    if let Some(icon) = with(|s| s.icon) {
        if !icon.is_null() {
            DestroyIcon(icon);
        }
    }
}

static CTL: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

/// Đọc lại thiết bị rồi cập nhật hiển thị.
unsafe fn refresh() {
    let devices = battery::snapshot();
    log!(
        "snapshot: {} devices: {}",
        devices.len(),
        devices
            .iter()
            .map(|d| format!("{} {}% {}", d.name, d.level, if d.connected { "connected" } else { "disconnected" }))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let stored = with(|s| s.devices = devices);
    if stored.is_none() {
        log!("snapshot: state busy, devices not stored");
    }
    update_view(false);
}

fn lines_for(s: &State) -> Vec<Line> {
    s.devices
        .iter()
        .filter(|d| d.connected || s.show_all)
        .map(|d| Line {
            name: d.name.clone(),
            pct: format!("{}%", d.level),
            glyph: overlay::battery_glyph(d.level),
            color: if !d.connected {
                [200, 200, 200]
            } else if d.level <= LOW_BATTERY {
                [255, 150, 130]
            } else {
                [255, 255, 255]
            },
        })
        .collect()
}

/// Vẽ lại overlay nếu nội dung hoặc trạng thái hiện/ẩn đổi (hoặc `force`).
unsafe fn update_view(force: bool) {
    let Some((overlay, ctl, lines, visible, drawn, tip, icon, left)) = with(|s| {
        let lines = lines_for(s);
        let visible = s.show && !s.fullscreen && !lines.is_empty();
        let tip = if s.devices.is_empty() {
            "screen-hud: no devices".to_string()
        } else {
            s.devices
                .iter()
                .map(|d| format!("{} {}%{}", d.name, d.level, if d.connected { "" } else { " (disconnected)" }))
                .collect::<Vec<_>>()
                .join("\n")
        };
        (s.overlay, s.ctl, lines, visible, s.drawn.clone(), tip, s.icon, s.left)
    }) else {
        log!("update_view: state busy, skipped");
        return;
    };
    log!("update_view: force={} visible={} lines={} drawn={}", force, visible, lines.len(), drawn.is_some());

    tray::set(ctl, WM_APP_TRAY, &tip, icon, false);

    if !visible {
        if drawn.is_some() {
            ShowWindow(overlay, SW_HIDE);
            with(|s| s.drawn = None);
        }
    } else if force || drawn.as_ref() != Some(&lines) {
        let r = overlay::render(overlay, &lines, left);
        log!("render: {:?}", r);
        ShowWindow(overlay, SW_SHOWNOACTIVATE);
        SetWindowPos(overlay, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        with(|s| s.drawn = Some(lines));
    }

    // Trả lại các trang bộ nhớ không dùng; process nền phần lớn thời gian chỉ ngủ trong GetMessage.
    SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
}

unsafe fn check_fullscreen() {
    let mut st: QUERY_USER_NOTIFICATION_STATE = 0;
    if SHQueryUserNotificationState(&mut st) < 0 {
        return;
    }
    let fs = st == QUNS_BUSY || st == QUNS_RUNNING_D3D_FULL_SCREEN || st == QUNS_PRESENTATION_MODE;
    if with(|s| std::mem::replace(&mut s.fullscreen, fs) != fs) == Some(true) {
        log!("fullscreen state={} -> hidden={}", st, fs);
        update_view(false);
    }
}

unsafe fn show_menu(hwnd: HWND) {
    let Some((show, left)) = with(|s| (s.show, s.left)) else { return };
    let autostart = install::autostart();
    let title = format!("screen-hud v{}", install::VERSION);
    let chosen = tray::menu(
        hwnd,
        &[
            tray::Item::Label(&title),
            tray::Item::Separator,
            tray::Item::Check(MENU_SHOW, "Show overlay", show),
            tray::Item::Check(MENU_LEFT, "Bottom left", left),
            tray::Item::Check(MENU_RIGHT, "Bottom right", !left),
            tray::Item::Check(MENU_AUTOSTART, "Start with Windows", autostart),
            tray::Item::Action(MENU_RELEASES, "Download updates…"),
            tray::Item::Separator,
            tray::Item::Action(MENU_EXIT, "Exit"),
        ],
    );
    match chosen {
        MENU_SHOW => {
            with(|s| s.show = !show);
            install::set_show_overlay(!show);
            update_view(false);
        }
        MENU_LEFT | MENU_RIGHT => {
            let left = chosen == MENU_LEFT;
            with(|s| s.left = left);
            install::set_overlay_left(left);
            update_view(true);
        }
        MENU_AUTOSTART => install::set_autostart(!autostart),
        MENU_RELEASES => {
            let (verb, url) = (wide("open"), wide(install::RELEASES_URL));
            ShellExecuteW(null_mut(), verb.as_ptr(), url.as_ptr(), null(), null(), SW_SHOWNORMAL);
        }
        MENU_EXIT => {
            DestroyWindow(hwnd);
        }
        _ => {}
    }
}

unsafe extern "system" fn ctl_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_APP_DEVICES => {
            // Lúc khởi tạo query và khi thiết bị kết nối, DevQuery gửi dồn nhiều thông báo.
            SetTimer(hwnd, TIMER_DEBOUNCE, 300, None);
            0
        }
        WM_TIMER => {
            match wp {
                TIMER_DEBOUNCE => {
                    KillTimer(hwnd, TIMER_DEBOUNCE);
                    refresh();
                }
                TIMER_POLL => refresh(),
                TIMER_FULLSCREEN => check_fullscreen(),
                _ => {}
            }
            0
        }
        WM_APP_TRAY => {
            let ev = lp as u32;
            if ev == WM_RBUTTONUP || ev == WM_LBUTTONUP || ev == WM_CONTEXTMENU {
                show_menu(hwnd);
            }
            0
        }
        WM_APP_REDRAW | WM_DISPLAYCHANGE | WM_SETTINGCHANGE => {
            // Đổi độ phân giải, DPI, vị trí/kích thước taskbar.
            update_view(true);
            if msg == WM_SETTINGCHANGE {
                DefWindowProcW(hwnd, msg, wp, lp)
            } else {
                0
            }
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => {
            if with(|s| s.taskbar_created) == Some(msg) && msg != 0 {
                // Explorer khởi động lại: thêm lại icon khay.
                let icon = with(|s| s.icon).unwrap_or(null_mut());
                tray::set(hwnd, WM_APP_TRAY, "screen-hud", icon, true);
                update_view(false);
                return 0;
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
    }
}

unsafe extern "system" fn overlay_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_NCHITTEST => HTTRANSPARENT as LRESULT,
        WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
        WM_DPICHANGED => {
            let ctl = CTL.load(std::sync::atomic::Ordering::Relaxed);
            if ctl != 0 {
                PostMessageW(ctl as HWND, WM_APP_REDRAW, 0, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
