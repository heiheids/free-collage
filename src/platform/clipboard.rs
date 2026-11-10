use std::ffi::c_void;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

use windows_sys::Win32::Foundation::{HANDLE, HWND};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW,
};
use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows_sys::Win32::UI::Shell::{DragQueryFileW, HDROP};

use super::system::wide;

const CF_DIB: u32 = 8;
const CF_DIBV5: u32 = 17;
const CF_HDROP: u32 = 15;

pub enum ClipboardContent {
    Encoded(Vec<u8>),
    Files(Vec<PathBuf>),
}

pub fn read(hwnd: HWND) -> Result<Vec<ClipboardContent>, String> {
    let mut opened = false;
    for _ in 0..6 {
        unsafe {
            if OpenClipboard(hwnd) != 0 {
                opened = true;
                break;
            }
        }
        sleep(Duration::from_millis(12));
    }
    if !opened {
        return Err("clipboard busy".into());
    }
    let result = read_inner();
    unsafe {
        CloseClipboard();
    }
    result
}

fn read_inner() -> Result<Vec<ClipboardContent>, String> {
    let png_format = unsafe { RegisterClipboardFormatW(wide("PNG").as_ptr()) };
    let mut out = Vec::new();
    unsafe {
        if png_format != 0 && IsClipboardFormatAvailable(png_format) != 0 {
            if let Some(bytes) = read_bytes(png_format) {
                if !bytes.is_empty() {
                    out.push(ClipboardContent::Encoded(bytes));
                    return Ok(out);
                }
            }
        }
        for format in [CF_DIBV5, CF_DIB] {
            if IsClipboardFormatAvailable(format) != 0 {
                if let Some(dib) = read_bytes(format) {
                    if let Some(bmp) = dib_to_bmp(&dib) {
                        out.push(ClipboardContent::Encoded(bmp));
                        return Ok(out);
                    }
                }
            }
        }
        if IsClipboardFormatAvailable(CF_HDROP) != 0 {
            let handle = GetClipboardData(CF_HDROP);
            if !handle.is_null() {
                let count = DragQueryFileW(handle as HDROP, u32::MAX, std::ptr::null_mut(), 0);
                for i in 0..count {
                    let len = DragQueryFileW(handle as HDROP, i, std::ptr::null_mut(), 0);
                    let mut buf = vec![0u16; len as usize + 1];
                    DragQueryFileW(handle as HDROP, i, buf.as_mut_ptr(), buf.len() as u32);
                    let path = String::from_utf16_lossy(&buf[..len as usize]);
                    out.push(ClipboardContent::Files(vec![PathBuf::from(path)]));
                }
            }
        }
    }
    if out.is_empty() {
        Err("empty".into())
    } else {
        Ok(out)
    }
}

unsafe fn read_bytes(format: u32) -> Option<Vec<u8>> {
    let handle: HANDLE = GetClipboardData(format);
    if handle.is_null() {
        return None;
    }
    let size = GlobalSize(handle);
    if size == 0 {
        return None;
    }
    let ptr = GlobalLock(handle) as *const u8;
    if ptr.is_null() {
        return None;
    }
    let bytes = std::slice::from_raw_parts(ptr, size).to_vec();
    GlobalUnlock(handle);
    Some(bytes)
}

fn read_u16(data: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?))
}

fn read_u32(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

fn dib_to_bmp(dib: &[u8]) -> Option<Vec<u8>> {
    if dib.len() < 40 {
        return None;
    }
    let header_size = read_u32(dib, 0)? as usize;
    if header_size < 40 || header_size > dib.len() {
        return None;
    }
    let bit_count = read_u16(dib, 14)?;
    let compression = read_u32(dib, 16)?;
    let clr_used = read_u32(dib, 32)? as usize;
    let palette = if bit_count <= 8 {
        let count = if clr_used == 0 { 1usize << bit_count } else { clr_used };
        count * 4
    } else {
        0
    };
    let masks = if compression == 3 && header_size == 40 { 12 } else { 0 };
    let off = 14 + header_size + masks + palette;
    let total = 14 + dib.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(off as u32).to_le_bytes());
    out.extend_from_slice(dib);
    Some(out)
}

pub fn clipboard_image_format_available() -> bool {
    let png = unsafe { RegisterClipboardFormatW(wide("PNG").as_ptr()) };
    unsafe {
        (png != 0 && IsClipboardFormatAvailable(png) != 0)
            || IsClipboardFormatAvailable(CF_DIBV5) != 0
            || IsClipboardFormatAvailable(CF_DIB) != 0
            || IsClipboardFormatAvailable(CF_HDROP) != 0
    }
}

pub fn _unused(_: *mut c_void) {}
