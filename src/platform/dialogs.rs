use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::{HWND, LPARAM};
use windows_sys::Win32::UI::Controls::Dialogs::{
    ChooseColorW, GetOpenFileNameW, GetSaveFileNameW, CHOOSECOLORW, OPENFILENAMEW,
    CC_ANYCOLOR, CC_FULLOPEN, CC_RGBINIT, OFN_ALLOWMULTISELECT, OFN_EXPLORER,
    OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST,
};

use super::system::wide;
use crate::model::style::Rgba;

fn filter_string() -> Vec<u16> {
    let s = "Images\0*.png;*.jpg;*.jpeg;*.webp;*.gif;*.bmp\0PNG\0*.png\0JPEG\0*.jpg;*.jpeg\0WebP\0*.webp\0GIF\0*.gif\0BMP\0*.bmp\0All files\0*.*\0\0";
    s.encode_utf16().collect()
}

pub fn open_images(hwnd: HWND, initial_dir: &str) -> Vec<PathBuf> {
    let filter = filter_string();
    let title = wide("打开图片 / Open images");
    let initial = if initial_dir.is_empty() {
        Vec::new()
    } else {
        wide(initial_dir)
    };
    let mut file_buf = vec![0u16; 65536];
    let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = hwnd;
    ofn.lpstrFilter = filter.as_ptr();
    ofn.nFilterIndex = 1;
    ofn.lpstrFile = file_buf.as_mut_ptr();
    ofn.nMaxFile = file_buf.len() as u32;
    ofn.lpstrInitialDir = if initial.is_empty() {
        std::ptr::null()
    } else {
        initial.as_ptr()
    };
    ofn.lpstrTitle = title.as_ptr();
    ofn.Flags = OFN_EXPLORER
        | OFN_FILEMUSTEXIST
        | OFN_PATHMUSTEXIST
        | OFN_ALLOWMULTISELECT
        | OFN_NOCHANGEDIR;
    let ok = unsafe { GetOpenFileNameW(&mut ofn) };
    if ok == 0 {
        return Vec::new();
    }
    parse_multi(&file_buf)
}

fn parse_multi(buf: &[u16]) -> Vec<PathBuf> {
    let mut parts: Vec<String> = Vec::new();
    let mut start = 0;
    for i in 0..buf.len() {
        if buf[i] == 0 {
            if i == start {
                break;
            }
            parts.push(String::from_utf16_lossy(&buf[start..i]));
            start = i + 1;
        }
    }
    if parts.is_empty() {
        return Vec::new();
    }
    if parts.len() == 1 {
        return vec![PathBuf::from(&parts[0])];
    }
    let base = PathBuf::from(&parts[0]);
    if base.is_dir() {
        parts[1..].iter().map(|p| base.join(p)).collect()
    } else {
        parts.into_iter().map(PathBuf::from).collect()
    }
}

pub fn save_image(hwnd: HWND, initial_dir: &str, default_name: &str, format: &str) -> Option<PathBuf> {
    let filter = filter_string();
    let title = wide("导出图片 / Export image");
    let initial = if initial_dir.is_empty() {
        Vec::new()
    } else {
        wide(initial_dir)
    };
    let name = wide(default_name);
    let ext = wide(match format {
        "jpeg" => "jpg",
        "webp" => "webp",
        "bmp" => "bmp",
        _ => "png",
    });
    let mut file_buf = vec![0u16; 1024];
    for (i, c) in name.iter().enumerate().take(file_buf.len() - 1) {
        file_buf[i] = *c;
    }
    let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = hwnd;
    ofn.lpstrFilter = filter.as_ptr();
    ofn.nFilterIndex = match format {
        "jpeg" => 3,
        "webp" => 4,
        "bmp" => 6,
        _ => 2,
    };
    ofn.lpstrFile = file_buf.as_mut_ptr();
    ofn.nMaxFile = file_buf.len() as u32;
    ofn.lpstrInitialDir = if initial.is_empty() {
        std::ptr::null()
    } else {
        initial.as_ptr()
    };
    ofn.lpstrTitle = title.as_ptr();
    ofn.lpstrDefExt = ext.as_ptr();
    ofn.Flags = OFN_EXPLORER
        | OFN_PATHMUSTEXIST
        | OFN_OVERWRITEPROMPT
        | OFN_NOCHANGEDIR;
    let ok = unsafe { GetSaveFileNameW(&mut ofn) };
    if ok == 0 {
        return None;
    }
    let path = crate::platform::system::from_wide(&file_buf);
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

pub fn choose_color(hwnd: HWND, initial: Rgba, custom: &mut [u32; 16]) -> Option<Rgba> {
    let mut cc: CHOOSECOLORW = unsafe { std::mem::zeroed() };
    cc.lStructSize = std::mem::size_of::<CHOOSECOLORW>() as u32;
    cc.hwndOwner = hwnd;
    cc.rgbResult = initial.to_colorref();
    cc.lpCustColors = custom.as_mut_ptr();
    cc.Flags = CC_FULLOPEN | CC_RGBINIT | CC_ANYCOLOR;
    cc.lCustData = 0 as LPARAM;
    let ok = unsafe { ChooseColorW(&mut cc) };
    if ok == 0 {
        None
    } else {
        Some(Rgba::from_colorref(cc.rgbResult))
    }
}

pub fn _unused(_: &Path) {}
