//! Đọc % pin thiết bị Bluetooth từ chính dữ liệu Windows lưu (cái mà Settings > Bluetooth hiển thị).
//!
//! - % pin: property PnP `{104EA319-6EE2-4701-BD47-8DDBF425BBE5} 2` trên devnode của thiết bị
//!   (tai nghe qua HFP, bàn phím/chuột BLE). Đây là giá trị lần cuối Windows nhận được, kể cả
//!   khi thiết bị đã ngắt kết nối, nên phải ghép với trạng thái kết nối.
//! - Kết nối + tên: `System.Devices.Connected` / `System.ItemNameDisplay` trên Device Container
//!   (mô hình "Devices and Printers"), ghép với devnode theo ContainerId.
//!
//! Không dùng Association Endpoint (AEP): truy vấn AEP Bluetooth chạy một lượt quét sóng
//! (~30 s mỗi giao thức) trước khi trả kết quả. Truy vấn container chỉ đọc dữ liệu có sẵn.
//! Dùng DevQuery (Win32, cfgmgr32) thay vì WinRT để không kéo thêm runtime/DLL nặng.

use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

use windows_sys::core::GUID;
use windows_sys::Win32::Devices::DeviceQuery::*;
use windows_sys::Win32::Devices::Properties::*;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW;

const fn key(fmtid: u128, pid: u32) -> DEVPROPKEY {
    DEVPROPKEY { fmtid: GUID::from_u128(fmtid), pid }
}

const BATTERY_LEVEL: DEVPROPKEY = key(0x104EA319_6EE2_4701_BD47_8DDBF425BBE5, 2);
const BATTERY_TIME: DEVPROPKEY = key(0x104EA319_6EE2_4701_BD47_8DDBF425BBE5, 7);
const DEVICE_CONTAINER_ID: DEVPROPKEY = key(0x8C7ED206_3F8A_4827_B3AB_AE9E1FAEFC6C, 2);
const CONTAINER_CONNECTED: DEVPROPKEY = key(0x78C34FC8_104A_4ACA_9EA4_524D52996E57, 55);
const ITEM_NAME_DISPLAY: DEVPROPKEY = key(0xB725F130_47EF_101A_A5F1_02608C9EEBAC, 10);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Device {
    pub name: String,
    pub level: u8,
    pub connected: bool,
}

fn battery_filter() -> [DEVPROP_FILTER_EXPRESSION; 1] {
    [filter(DEVPROP_OPERATOR_EXISTS, BATTERY_LEVEL, DEVPROP_TYPE_EMPTY)]
}

/// Danh sách thiết bị có % pin, sắp xếp: đang kết nối trước, rồi theo tên.
pub fn snapshot() -> Vec<Device> {
    snapshot_timed().0
}

/// Như [`snapshot`], kèm thời gian của truy vấn devnode và truy vấn container.
pub fn snapshot_timed() -> (Vec<Device>, std::time::Duration, std::time::Duration) {
    let t0 = std::time::Instant::now();
    // ContainerId -> (level, thời điểm cập nhật). Một thiết bị có thể có vài devnode mang pin,
    // lấy giá trị mới nhất.
    let mut levels: HashMap<u128, (u8, u64)> = HashMap::new();
    query(
        DevObjectTypeDevice,
        &[BATTERY_LEVEL, BATTERY_TIME, DEVICE_CONTAINER_ID],
        &battery_filter(),
        |_, props| {
            let (Some(level), Some(container)) = (
                find(props, &BATTERY_LEVEL).and_then(as_u8),
                find(props, &DEVICE_CONTAINER_ID).and_then(as_guid),
            ) else {
                return;
            };
            let time = find(props, &BATTERY_TIME).and_then(as_u64).unwrap_or(0);
            let entry = levels.entry(container).or_insert((level, time));
            if time >= entry.1 {
                *entry = (level, time);
            }
        },
    );

    let t1 = std::time::Instant::now();
    let mut list = Vec::new();
    if !levels.is_empty() {
        query(DevObjectTypeDeviceContainer, &[ITEM_NAME_DISPLAY, CONTAINER_CONNECTED], &[], |id, props| {
            let Some(container) = parse_guid(id) else { return };
            let Some(&(level, _)) = levels.get(&container) else { return };
            let name = find(props, &ITEM_NAME_DISPLAY)
                .and_then(as_string)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            if name.is_empty() {
                return;
            }
            let connected = find(props, &CONTAINER_CONNECTED).and_then(as_bool).unwrap_or(false);
            list.push(Device { name, level, connected });
        });
    }
    list.sort_by(|a, b| b.connected.cmp(&a.connected).then_with(|| a.name.cmp(&b.name)));
    (list, t1 - t0, t1.elapsed())
}

/// Đăng ký nhận thông báo khi % pin hoặc trạng thái kết nối thay đổi; mỗi thay đổi post `msg`
/// tới `hwnd`. Trả về các handle query, cần giữ sống và đóng bằng [`unwatch`].
pub fn watch(hwnd: HWND, msg: u32) -> Vec<HDEVQUERY> {
    WATCH_MSG.store(msg, std::sync::atomic::Ordering::Relaxed);
    let battery = battery_filter();
    let targets: [(DEV_OBJECT_TYPE, &[DEVPROPKEY], &[DEVPROP_FILTER_EXPRESSION]); 2] = [
        (DevObjectTypeDevice, &[BATTERY_LEVEL, BATTERY_TIME], &battery),
        (DevObjectTypeDeviceContainer, &[CONTAINER_CONNECTED], &[]),
    ];
    let mut handles = Vec::new();
    for (kind, keys, f) in targets {
        let comp = compkeys(keys);
        let mut h: HDEVQUERY = null_mut();
        let hr = unsafe {
            DevCreateObjectQuery(
                kind,
                DevQueryFlagUpdateResults as u32,
                comp.len() as u32,
                comp.as_ptr(),
                f.len() as u32,
                if f.is_empty() { null() } else { f.as_ptr() },
                Some(on_change),
                hwnd as *const c_void,
                &mut h,
            )
        };
        if hr >= 0 && !h.is_null() {
            handles.push(h);
        }
    }
    handles
}

pub fn unwatch(handles: &[HDEVQUERY]) {
    for &h in handles {
        unsafe { DevCloseObjectQuery(h) };
    }
}

static WATCH_MSG: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

unsafe extern "system" fn on_change(_q: HDEVQUERY, ctx: *const c_void, _data: *const DEV_QUERY_RESULT_ACTION_DATA) {
    // Chạy trên thread pool: chỉ báo cho UI thread, UI thread tự debounce rồi đọc lại.
    PostMessageW(ctx as HWND, WATCH_MSG.load(std::sync::atomic::Ordering::Relaxed), 0, 0);
}

fn compkeys(keys: &[DEVPROPKEY]) -> Vec<DEVPROPCOMPKEY> {
    keys.iter()
        .map(|k| DEVPROPCOMPKEY { Key: *k, Store: DEVPROP_STORE_SYSTEM, LocaleName: null() })
        .collect()
}

fn filter(op: DEVPROP_OPERATOR, k: DEVPROPKEY, ty: DEVPROPTYPE) -> DEVPROP_FILTER_EXPRESSION {
    DEVPROP_FILTER_EXPRESSION {
        Operator: op,
        Property: DEVPROPERTY {
            CompKey: DEVPROPCOMPKEY { Key: k, Store: DEVPROP_STORE_SYSTEM, LocaleName: null() },
            Type: ty,
            BufferSize: 0,
            Buffer: null_mut(),
        },
    }
}

/// Chạy `DevGetObjects`, gọi `f(object id, properties)` cho từng object.
fn query(
    kind: DEV_OBJECT_TYPE,
    keys: &[DEVPROPKEY],
    filters: &[DEVPROP_FILTER_EXPRESSION],
    mut f: impl FnMut(&[u16], &[DEVPROPERTY]),
) {
    let comp = compkeys(keys);
    let mut count: u32 = 0;
    let mut objs: *mut DEV_OBJECT = null_mut();
    let hr = unsafe {
        DevGetObjects(
            kind,
            DevQueryFlagNone as u32,
            comp.len() as u32,
            comp.as_ptr(),
            filters.len() as u32,
            if filters.is_empty() { null() } else { filters.as_ptr() },
            &mut count,
            &mut objs as *mut *mut DEV_OBJECT as _,
        )
    };
    if hr < 0 || objs.is_null() {
        return;
    }
    unsafe {
        for o in std::slice::from_raw_parts(objs, count as usize) {
            if o.pProperties.is_null() || o.pszObjectId.is_null() {
                continue;
            }
            let mut n = 0;
            while *o.pszObjectId.add(n) != 0 {
                n += 1;
            }
            f(
                std::slice::from_raw_parts(o.pszObjectId, n),
                std::slice::from_raw_parts(o.pProperties, o.cPropertyCount as usize),
            );
        }
        DevFreeObjects(count, objs);
    }
}

fn same_key(a: &DEVPROPKEY, b: &DEVPROPKEY) -> bool {
    a.pid == b.pid
        && a.fmtid.data1 == b.fmtid.data1
        && a.fmtid.data2 == b.fmtid.data2
        && a.fmtid.data3 == b.fmtid.data3
        && a.fmtid.data4 == b.fmtid.data4
}

fn find<'a>(props: &'a [DEVPROPERTY], k: &DEVPROPKEY) -> Option<&'a DEVPROPERTY> {
    props
        .iter()
        .find(|p| same_key(&p.CompKey.Key, k) && p.Type != DEVPROP_TYPE_EMPTY && !p.Buffer.is_null())
}

fn bytes(p: &DEVPROPERTY) -> &[u8] {
    unsafe { std::slice::from_raw_parts(p.Buffer as *const u8, p.BufferSize as usize) }
}

fn as_u8(p: &DEVPROPERTY) -> Option<u8> {
    (p.Type == DEVPROP_TYPE_BYTE).then(|| bytes(p).first().copied()).flatten()
}

fn as_bool(p: &DEVPROPERTY) -> Option<bool> {
    (p.Type == DEVPROP_TYPE_BOOLEAN).then(|| bytes(p).first().map(|&b| b != 0)).flatten()
}

fn as_u64(p: &DEVPROPERTY) -> Option<u64> {
    let b = bytes(p);
    (p.Type == DEVPROP_TYPE_FILETIME && b.len() >= 8).then(|| u64::from_le_bytes(b[..8].try_into().unwrap()))
}

/// GUID dạng số 128-bit theo thứ tự chữ viết ("{data1-data2-data3-data4}"), để so khớp với
/// [`parse_guid`].
fn as_guid(p: &DEVPROPERTY) -> Option<u128> {
    let b = bytes(p);
    if p.Type != DEVPROP_TYPE_GUID || b.len() < 16 {
        return None;
    }
    let d1 = u32::from_le_bytes(b[0..4].try_into().unwrap()) as u128;
    let d2 = u16::from_le_bytes(b[4..6].try_into().unwrap()) as u128;
    let d3 = u16::from_le_bytes(b[6..8].try_into().unwrap()) as u128;
    let d4 = u64::from_be_bytes(b[8..16].try_into().unwrap()) as u128;
    Some(d1 << 96 | d2 << 80 | d3 << 64 | d4)
}

/// Object id của container có dạng "{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}".
fn parse_guid(id: &[u16]) -> Option<u128> {
    let s = String::from_utf16_lossy(id);
    let hex: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    (hex.len() == 32).then(|| u128::from_str_radix(&hex, 16).ok()).flatten()
}

fn as_string(p: &DEVPROPERTY) -> Option<String> {
    if p.Type != DEVPROP_TYPE_STRING {
        return None;
    }
    let units: Vec<u16> = bytes(p).chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
    Some(String::from_utf16_lossy(&units[..end]))
}
