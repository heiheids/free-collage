use std::ffi::c_void;
use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::{HWND, SYSTEMTIME};
use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::UI::HiDpi::{
    GetDpiForSystem, GetDpiForWindow, SetProcessDpiAwarenessContext,
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, LoadImageW, SM_CXPADDEDBORDER, SM_CXSIZEFRAME, SM_CYSIZEFRAME,
    IMAGE_ICON, LR_DEFAULTCOLOR, SW_SHOWNORMAL,
};

pub const APP_ICON_ID: usize = 101;

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

pub fn set_dpi_awareness() {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

pub fn window_dpi(hwnd: HWND) -> f32 {
    unsafe {
        let dpi = GetDpiForWindow(hwnd);
        if dpi > 0 {
            dpi as f32
        } else {
            96.0
        }
    }
}

pub fn system_dpi() -> f32 {
    unsafe {
        let dpi = GetDpiForSystem();
        if dpi > 0 {
            dpi as f32
        } else {
            96.0
        }
    }
}

pub fn system_language() -> String {
    let mut buf = [0u16; 85];
    unsafe {
        let len = GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32);
        if len > 0 {
            from_wide(&buf)
        } else {
            "en-US".into()
        }
    }
}

pub fn exe_dir() -> PathBuf {
    let mut buf = vec![0u16; 32768];
    unsafe {
        let len = GetModuleFileNameW(std::ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32);
        if len > 0 {
            let path = PathBuf::from(String::from_utf16_lossy(&buf[..len as usize]));
            if let Some(dir) = path.parent() {
                return dir.to_path_buf();
            }
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn settings_path() -> PathBuf {
    exe_dir().join("settings.cfg")
}

pub fn asset_path(name: &str) -> PathBuf {
    exe_dir().join("assets").join(name)
}

pub fn format_bytes(n: u64) -> String {
    if n >= 1024 * 1024 {
        format!("{:.1} MB", n as f64 / (1024.0 * 1024.0))
    } else if n >= 1024 {
        format!("{:.0} KB", n as f64 / 1024.0)
    } else {
        format!("{n} B")
    }
}

pub fn timestamp_for_filename() -> String {
    unsafe {
        let mut st: SYSTEMTIME = std::mem::zeroed();
        GetLocalTime(&mut st);
        format!(
            "{:04}{:02}{:02}_{:02}{:02}{:02}",
            st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond
        )
    }
}

pub fn open_url(url: &str) {
    let op = wide("open");
    let target = wide(url);
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            op.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}

pub fn load_app_icon(hinstance: *mut c_void, small: bool) -> *mut c_void {
    unsafe {
        let size = if small { 16 } else { 32 };
        let icon = LoadImageW(
            hinstance,
            APP_ICON_ID as *const u16,
            IMAGE_ICON,
            size,
            size,
            LR_DEFAULTCOLOR,
        );
        if !icon.is_null() {
            return icon;
        }
        LoadImageW(
            hinstance,
            APP_ICON_ID as *const u16,
            IMAGE_ICON,
            0,
            0,
            LR_DEFAULTCOLOR,
        )
    }
}

pub fn load_icon_from_asset(small: bool) -> *mut c_void {
    let path = asset_path("app.ico");
    if !path.exists() {
        return std::ptr::null_mut();
    }
    let w = wide(&path.to_string_lossy());
    unsafe {
        LoadImageW(
            std::ptr::null_mut(),
            w.as_ptr(),
            IMAGE_ICON,
            if small { 16 } else { 32 },
            if small { 16 } else { 32 },
            LR_DEFAULTCOLOR,
        )
    }
}

pub fn round_window_corners(hwnd: HWND) {
    let preference: u32 = 2; // DWMWCP_ROUND
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            33, // DWMWA_WINDOW_CORNER_PREFERENCE
            &preference as *const u32 as *const c_void,
            std::mem::size_of::<u32>() as u32,
        );
    }
}

pub fn work_area(hwnd: HWND) -> windows_sys::Win32::Foundation::RECT {
    unsafe {
        let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if !mon.is_null() && GetMonitorInfoW(mon, &mut info) != 0 {
            info.rcWork
        } else {
            windows_sys::Win32::Foundation::RECT {
                left: 0,
                top: 0,
                right: GetSystemMetrics(0),
                bottom: GetSystemMetrics(1),
            }
        }
    }
}

pub fn resize_border() -> (i32, i32) {
    unsafe {
        (
            GetSystemMetrics(SM_CXSIZEFRAME) + GetSystemMetrics(SM_CXPADDEDBORDER),
            GetSystemMetrics(SM_CYSIZEFRAME) + GetSystemMetrics(SM_CXPADDEDBORDER),
        )
    }
}

pub fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "image".into())
}

pub fn default_export_name(format: &str) -> String {
    format!(
        "自由拼图_{}.{}",
        timestamp_for_filename(),
        match format {
            "jpeg" => "jpg",
            "webp" => "webp",
            "bmp" => "bmp",
            _ => "png",
        }
    )
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
