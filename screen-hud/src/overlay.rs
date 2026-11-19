//! Cửa sổ overlay: layered, click xuyên qua, không lấy focus, không hiện trên taskbar/Alt-Tab.
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

/// Độ mờ tổng thể (0-255) áp lên toàn bộ overlay.
const OPACITY: u8 = 215;
/// Độ đậm nền mờ phía sau chữ (0.0 = không nền).
const BACKDROP: f32 = 0.28;
const SHADOW: f32 = 0.5;

/// Vẽ overlay ở góc dưới phải (hoặc trái nếu `left`) vùng làm việc; trả về (x, y, w, h) đã đặt, hoặc chuỗi mô tả bước thất bại.
pub unsafe fn render(hwnd: HWND, lines: &[Line], left: bool) -> Result<(i32, i32, i32, i32), String> {
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
    let name_w = names.iter().map(|t| extent(mem, t).cx).max().unwrap_or(0);
    let pct_w = pcts.iter().map(|t| extent(mem, t).cx).max().unwrap_or(0);
    SelectObject(mem, icon_font);
    let mut itm: TEXTMETRICW = std::mem::zeroed();
    GetTextMetricsW(mem, &mut itm);
    let glyph_w = lines.iter().map(|l| extent(mem, &[l.glyph]).cx).max().unwrap_or(0);

    let pad_x = s(8);
    let pad_y = s(5);
    let gap = s(10);
    let w = pad_x * 2 + name_w + gap + glyph_w + s(4) + pct_w;
    let h = pad_y * 2 + line_h * lines.len() as i32 - s(1);

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
        let y = pad_y + line_h * i as i32;
        SelectObject(mem, font);
        TextOutW(mem, pad_x, y, names[i].as_ptr(), names[i].len() as i32);
        let pw = extent(mem, &pcts[i]).cx;
        TextOutW(mem, w - pad_x - pw, y, pcts[i].as_ptr(), pcts[i].len() as i32);
        SelectObject(mem, icon_font);
        let gx = w - pad_x - pct_w - s(4) - glyph_w;
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
    for y in 0..hu {
        let li = ((y as i32 - pad_y).max(0) / line_h).min(lines.len() as i32 - 1) as usize;
        let [r, g, b] = lines[li].color;
        for x in 0..wu {
            let mut sum = 0.0;
            let mut n = 0.0;
            for dy in -halo..=halo {
                for dx in -halo..=halo {
                    sum += at(x as isize + dx, y as isize + dy);
                    n += 1.0;
                }
            }
            let a_s = (sum / n / 255.0 * 2.0).min(1.0) * SHADOW;
            let a_t = mask[y * wu + x] as f32 / 255.0;
            let a_b = BACKDROP * rounded_coverage(x as f32 + 0.5, y as f32 + 0.5, w as f32, h as f32, radius);
            let a = 1.0 - (1.0 - a_b) * (1.0 - a_s) * (1.0 - a_t);
            let pm = |c: u8| (c as f32 * a_t).round() as u32;
            px[y * wu + x] = ((a * 255.0).round() as u32) << 24 | pm(r) << 16 | pm(g) << 8 | pm(b);
        }
    }

    // Góc dưới phải/trái vùng làm việc (phía trên taskbar).
    let mut work: RECT = std::mem::zeroed();
    SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut RECT as *mut c_void, 0);
    let margin = s(6);
    let x = if left { work.left + margin } else { work.right - w - margin };
    let pos = POINT { x, y: work.bottom - h - margin };
    let size = SIZE { cx: w, cy: h };
    let src = POINT { x: 0, y: 0 };
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: OPACITY,
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
