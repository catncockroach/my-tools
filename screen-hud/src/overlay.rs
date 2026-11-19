//! Cửa sổ overlay: layered, click xuyên qua, không lấy focus, không hiện trên taskbar/Alt-Tab.
//! Đặt ở góc dưới phải/trái vùng làm việc, hoặc nằm trong vùng trống trên taskbar.
//!
//! Vẽ bằng GDI thuần (không Direct2D/GDI+ để không nạp thêm DLL): chữ trắng khử răng cưa
//! xám trên nền đen -> lấy độ sáng làm mask alpha -> tự ghép nền mờ + bóng + màu chữ thành
//! ảnh BGRA premultiplied -> `UpdateLayeredWindow`. Chỉ vẽ khi nội dung đổi; bitmap được giải
//! phóng ngay sau khi giao cho DWM.

use std::ffi::c_void;
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{COLORREF, HWND, POINT, RECT, SIZE};
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::Shell::{SHAppBarMessage, ABM_GETSTATE, ABS_AUTOHIDE, APPBARDATA};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::wide;

pub const EX_STYLE: u32 =
    WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Line {
    pub name: String,
    pub pct: String,
    /// Ký tự icon pin trong font Segoe Fluent Icons / Segoe MDL2 Assets.
    pub glyph: u16,
    pub color: [u8; 3],
}

/// Icon pin theo mức %: Battery0..Battery9 = U+E850..U+E859, Battery10 = U+E83F.
pub fn battery_glyph(level: u8) -> u16 {
    if level >= 95 {
        0xE83F
    } else {
        0xE850 + (level as u16 + 5) / 10
    }
}

/// Nơi đặt overlay.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Place {
    /// Góc dưới phải (`left` = false) hoặc dưới trái của vùng làm việc, có nền mờ.
    Corner { left: bool },
    /// Trong taskbar, không nền.
    Taskbar(Slot),
}

/// Vùng trống trên taskbar: overlay căn phải vào `right`, căn giữa theo chiều dọc trong `top..bottom`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Slot {
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
    /// Taskbar theme sáng: chữ tối.
    pub light: bool,
}

/// Chỗ trống bên trái nút Widgets (thời tiết) khi Windows 11 đặt nó cạnh khay hệ thống, ở 96 DPI.
/// Nút không có HWND riêng (là phần tử XAML), chỉ đo được qua UI Automation (nặng), nên dùng
/// bề rộng cố định: ~152 px nút + khoảng cách tới khay.
const WIDGETS_W: i32 = 164;

/// Tìm vùng trống trên taskbar chính: ngay bên trái khay hệ thống (và nút Widgets nếu có).
/// None nếu không có taskbar hoặc taskbar đang tự ẩn.
pub unsafe fn taskbar_slot() -> Option<Slot> {
    let tb = FindWindowW(wide("Shell_TrayWnd").as_ptr(), std::ptr::null());
    if tb.is_null() || IsWindowVisible(tb) == 0 {
        return None;
    }
    let mut abd: APPBARDATA = std::mem::zeroed();
    abd.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
    if SHAppBarMessage(ABM_GETSTATE, &mut abd) as u32 & ABS_AUTOHIDE != 0 {
        return None;
    }
    let mut r: RECT = std::mem::zeroed();
    GetWindowRect(tb, &mut r);
    let tray = FindWindowExW(tb, null_mut(), wide("TrayNotifyWnd").as_ptr(), std::ptr::null());
    let mut right = r.right;
    if !tray.is_null() {
        let mut t: RECT = std::mem::zeroed();
        GetWindowRect(tray, &mut t);
        right = t.left;
    }
    if crate::install::widgets_beside_tray() {
        right -= (WIDGETS_W * GetDpiForWindow(tb).max(96) as i32 + 48) / 96;
    }
    Some(Slot { right, top: r.top, bottom: r.bottom, light: crate::install::light_taskbar() })
}

/// Độ mờ tổng thể (0-255) áp lên toàn bộ overlay ở góc màn hình.
const OPACITY: u8 = 215;
/// Độ đậm nền mờ phía sau chữ (0.0 = không nền).
const BACKDROP: f32 = 0.28;
const SHADOW: f32 = 0.5;

/// Vẽ overlay tại `place`; trả về (x, y, w, h) đã đặt, hoặc chuỗi mô tả bước thất bại.
pub unsafe fn render(hwnd: HWND, lines: &[Line], place: Place) -> Result<(i32, i32, i32, i32), String> {
    let dpi = GetDpiForWindow(hwnd).max(96) as i32;
    let s = |v: i32| (v * dpi + 48) / 96;

    let screen = GetDC(null_mut());
    let mem = CreateCompatibleDC(screen);
    let font = make_font(s(12), FW_SEMIBOLD as i32, "Segoe UI");
    let icon_font = make_icon_font(mem, s(14));

    // Đo kích thước.
    let old_font = SelectObject(mem, font);
    let mut tm: TEXTMETRICW = std::mem::zeroed();
    GetTextMetricsW(mem, &mut tm);
    let line_h = tm.tmHeight + s(1);
    let names: Vec<Vec<u16>> = lines.iter().map(|l| wide_nonul(&l.name)).collect();
    let pcts: Vec<Vec<u16>> = lines.iter().map(|l| wide_nonul(&l.pct)).collect();
    let name_ws: Vec<i32> = names.iter().map(|t| extent(mem, t).cx).collect();
    let pct_w = pcts.iter().map(|t| extent(mem, t).cx).max().unwrap_or(0);
    SelectObject(mem, icon_font);
    let mut itm: TEXTMETRICW = std::mem::zeroed();
    GetTextMetricsW(mem, &mut itm);
    let glyph_w = lines.iter().map(|l| extent(mem, &[l.glyph]).cx).max().unwrap_or(0);

    let pad_x = s(8);
    let pad_y = if matches!(place, Place::Taskbar(_)) { s(2) } else { s(5) };
    let gap = s(10);
    let col_gap = s(16);
    // Ở góc: một cột. Trên taskbar: chia cột, mỗi cột tối đa số dòng vừa chiều cao taskbar.
    let count = lines.len().max(1) as i32;
    let rows = match place {
        Place::Corner { .. } => count,
        Place::Taskbar(t) => ((t.bottom - t.top - pad_y * 2 + s(1)) / line_h).clamp(1, count),
    };
    let cols = (count + rows - 1) / rows;
    // Mỗi cột rộng theo tên dài nhất trong cột đó; col_x[c] = mép trái cột c.
    let col_w: Vec<i32> = name_ws
        .chunks(rows as usize)
        .map(|c| c.iter().copied().max().unwrap_or(0) + gap + glyph_w + s(4) + pct_w)
        .collect();
    let col_x: Vec<i32> = col_w.iter().scan(pad_x, |x, cw| Some(std::mem::replace(x, *x + cw + col_gap))).collect();
    let w = pad_x * 2 + col_w.iter().sum::<i32>() + col_gap * (cols - 1);
    let h = pad_y * 2 + line_h * rows - s(1);

    // DIB 32bpp top-down.
    let mut bmi: BITMAPINFO = std::mem::zeroed();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = w;
    bmi.bmiHeader.biHeight = -h;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;
    let mut bits: *mut c_void = null_mut();
    let dib = CreateDIBSection(mem, &bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
    if dib.is_null() || bits.is_null() {
        SelectObject(mem, old_font);
        DeleteObject(font);
        DeleteObject(icon_font);
        DeleteDC(mem);
        ReleaseDC(null_mut(), screen);
        return Err(format!("CreateDIBSection {}x{} failed", w, h));
    }
    let old_bmp = SelectObject(mem, dib);
    let px = std::slice::from_raw_parts_mut(bits as *mut u32, (w * h) as usize);
    px.fill(0);

    // Vẽ chữ trắng lên nền đen để lấy mask.
    SetBkMode(mem, TRANSPARENT as i32);
    SetTextColor(mem, 0x00FF_FFFF as COLORREF);
    let icon_dy = (tm.tmHeight - itm.tmHeight) / 2;
    for (i, l) in lines.iter().enumerate() {
        let c = i / rows as usize;
        let (x, cw) = (col_x[c], col_w[c]);
        let y = pad_y + line_h * (i as i32 % rows);
        SelectObject(mem, font);
        TextOutW(mem, x, y, names[i].as_ptr(), names[i].len() as i32);
        let pw = extent(mem, &pcts[i]).cx;
        TextOutW(mem, x + cw - pw, y, pcts[i].as_ptr(), pcts[i].len() as i32);
        SelectObject(mem, icon_font);
        let gx = x + cw - pct_w - s(4) - glyph_w;
        TextOutW(mem, gx, y + icon_dy, &l.glyph, 1);
    }
    GdiFlush();

    let wu = w as usize;
    let hu = h as usize;
    let mask: Vec<u8> = px.iter().map(|&p| ((p >> 16) & 0xFF).max((p >> 8) & 0xFF).max(p & 0xFF) as u8).collect();

    // Viền tối mờ quanh chữ (không lệch) để đọc được trên cả nền sáng: lấy max của mask
    // trong bán kính `halo` rồi làm mềm theo khoảng cách.
    let halo = s(1).max(1) as isize;
    let at = |x: isize, y: isize| -> f32 {
        if x < 0 || y < 0 || x >= w as isize || y >= h as isize {
            0.0
        } else {
            mask[y as usize * wu + x as usize] as f32
        }
    };
    let radius = s(6) as f32;
    // Trên taskbar: không nền mờ; taskbar sáng thì chữ tối, không cần viền tối.
    let (backdrop, shadow) = match place {
        Place::Corner { .. } => (BACKDROP, SHADOW),
        Place::Taskbar(t) => (0.0, if t.light { 0.0 } else { SHADOW }),
    };
    let line_at = |x: usize, y: usize| -> usize {
        let col = col_x.iter().rposition(|&cx| cx <= x as i32).unwrap_or(0) as i32;
        let row = ((y as i32 - pad_y).max(0) / line_h).min(rows - 1);
        ((col * rows + row) as usize).min(lines.len() - 1)
    };
    for y in 0..hu {
        for x in 0..wu {
            let [r, g, b] = lines[line_at(x, y)].color;
            let mut sum = 0.0;
            let mut n = 0.0;
            for dy in -halo..=halo {
                for dx in -halo..=halo {
                    sum += at(x as isize + dx, y as isize + dy);
                    n += 1.0;
                }
            }
            let a_s = (sum / n / 255.0 * 2.0).min(1.0) * shadow;
            let a_t = mask[y * wu + x] as f32 / 255.0;
            let a_b = backdrop * rounded_coverage(x as f32 + 0.5, y as f32 + 0.5, w as f32, h as f32, radius);
            let a = 1.0 - (1.0 - a_b) * (1.0 - a_s) * (1.0 - a_t);
            let pm = |c: u8| (c as f32 * a_t).round() as u32;
            px[y * wu + x] = ((a * 255.0).round() as u32) << 24 | pm(r) << 16 | pm(g) << 8 | pm(b);
        }
    }

    let margin = s(6);
    let (pos, opacity) = match place {
        Place::Corner { left } => {
            // Góc dưới phải/trái vùng làm việc (phía trên taskbar).
            let mut work: RECT = std::mem::zeroed();
            SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut RECT as *mut c_void, 0);
            let x = if left { work.left + margin } else { work.right - w - margin };
            (POINT { x, y: work.bottom - h - margin }, OPACITY)
        }
        Place::Taskbar(t) => (POINT { x: t.right - w - margin, y: t.top + (t.bottom - t.top - h) / 2 }, 255),
    };
    let size = SIZE { cx: w, cy: h };
    let src = POINT { x: 0, y: 0 };
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: opacity,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };
    let ok = UpdateLayeredWindow(hwnd, screen, &pos, &size, mem, &src, 0, &blend, ULW_ALPHA);
    let err = windows_sys::Win32::Foundation::GetLastError();

    SelectObject(mem, old_bmp);
    SelectObject(mem, old_font);
    DeleteObject(dib);
    DeleteObject(font);
    DeleteObject(icon_font);
    DeleteDC(mem);
    ReleaseDC(null_mut(), screen);
    if ok == 0 {
        return Err(format!("UpdateLayeredWindow failed, GetLastError={}", err));
    }
    Ok((pos.x, pos.y, w, h))
}

/// Độ phủ (0..1) của hình chữ nhật bo góc tại điểm (x, y), khử răng cưa ở mép góc.
fn rounded_coverage(x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let cx = x.clamp(r, w - r);
    let cy = y.clamp(r, h - r);
    let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
    (r - d + 0.5).clamp(0.0, 1.0)
}

unsafe fn make_font(px: i32, weight: i32, face: &str) -> HFONT {
    let face = wide(face);
    CreateFontW(
        -px,
        0,
        0,
        0,
        weight,
        0,
        0,
        0,
        DEFAULT_CHARSET as u32,
        OUT_TT_PRECIS as u32,
        CLIP_DEFAULT_PRECIS as u32,
        ANTIALIASED_QUALITY as u32,
        (DEFAULT_PITCH | FF_DONTCARE) as u32,
        face.as_ptr(),
    )
}

/// Windows 11 có Segoe Fluent Icons; Windows 10 chỉ có Segoe MDL2 Assets (cùng mã ký tự pin).
unsafe fn make_icon_font(dc: HDC, px: i32) -> HFONT {
    for face in ["Segoe Fluent Icons", "Segoe MDL2 Assets"] {
        let f = make_font(px, FW_NORMAL as i32, face);
        let old = SelectObject(dc, f);
        let mut buf = [0u16; 64];
        let n = GetTextFaceW(dc, buf.len() as i32, buf.as_mut_ptr());
        SelectObject(dc, old);
        let got = String::from_utf16_lossy(&buf[..(n.max(1) - 1) as usize]);
        if got.eq_ignore_ascii_case(face) {
            return f;
        }
        DeleteObject(f);
    }
    make_font(px, FW_NORMAL as i32, "Segoe UI Symbol")
}

unsafe fn extent(dc: HDC, text: &[u16]) -> SIZE {
    let mut sz = SIZE { cx: 0, cy: 0 };
    GetTextExtentPoint32W(dc, text.as_ptr(), text.len() as i32, &mut sz);
    sz
}

fn wide_nonul(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
