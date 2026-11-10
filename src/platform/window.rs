use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicIsize, AtomicPtr, Ordering};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, GetMonitorInfoW, InvalidateRect, MonitorFromWindow, ScreenToClient,
    SetBkColor, UpdateWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST, PAINTSTRUCT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, SetFocus, TrackMouseEvent, TME_LEAVE,
    TRACKMOUSEEVENT, VK_CONTROL, VK_SHIFT,
};
use windows_sys::Win32::UI::Shell::{DragFinish, DragQueryFileW, HDROP};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetMessageW, IsZoomed, LoadCursorW, MSG, PostQuitMessage, RegisterClassExW, SendMessageW,
    SetCursor, SetWindowLongPtrW, SetWindowPos, ShowWindow, SystemParametersInfoW,
    TranslateMessage, CS_DBLCLKS, CS_HREDRAW, CS_VREDRAW, ES_AUTOVSCROLL, ES_MULTILINE,
    ES_WANTRETURN, GWLP_USERDATA, GWLP_WNDPROC, HTCLIENT, IDC_ARROW, IDC_CROSS, IDC_HAND,
    IDC_IBEAM, IDC_SIZEALL, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, MINMAXINFO,
    SPI_GETWORKAREA, SWP_NOACTIVATE, SWP_NOZORDER, SW_SHOW, WM_CHAR, WM_CLOSE, WM_COMMAND,
    WM_CTLCOLOREDIT, WM_DESTROY, WM_DPICHANGED, WM_DROPFILES, WM_ERASEBKGND, WM_GETMINMAXINFO,
    WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN,
    WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCALCSIZE, WM_NCDESTROY, WM_NCHITTEST,
    WM_PAINT, WM_PASTE, WM_PRINTCLIENT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETCURSOR, WM_SIZE,
    WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER,
    WNDCLASSEXW, WS_BORDER, WS_CHILD, WS_CLIPCHILDREN, WS_EX_APPWINDOW, WS_MAXIMIZEBOX,
    WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_TABSTOP, WS_THICKFRAME, WS_VISIBLE,
    WM_CAPTURECHANGED,
};

use crate::app::App;
use crate::platform::gdi::white_brush;
use crate::platform::system;
use crate::tools::Cursor;

static APP_PTR: AtomicPtr<App> = AtomicPtr::new(ptr::null_mut());
static EDIT_OLD_PROC: AtomicIsize = AtomicIsize::new(0);

const WM_MOUSELEAVE_MSG: u32 = 0x02A3;
const EM_SETSEL: u32 = 0x00B1;
const WM_SETFONT: u32 = 0x0030;
const SWP_NOZORDER_NOACTIVATE: u32 = SWP_NOZORDER | SWP_NOACTIVATE;

pub fn current_app() -> *mut App {
    APP_PTR.load(Ordering::Relaxed)
}

pub fn run() {
    unsafe {
        system::set_dpi_awareness();
        let hinstance = GetModuleHandleW(ptr::null());
        let class_name = system::wide("PuzzleStudio.Window");
        let title = system::wide("自由拼图 · Free Collage");

        let mut icon = system::load_app_icon(hinstance, false);
        if icon.is_null() {
            icon = system::load_icon_from_asset(false);
        }
        let mut icon_small = system::load_app_icon(hinstance, true);
        if icon_small.is_null() {
            icon_small = system::load_icon_from_asset(true);
        }
        let cursor = LoadCursorW(ptr::null_mut(), IDC_ARROW);

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: icon,
            hCursor: cursor,
            hbrBackground: ptr::null_mut(),
            lpszMenuName: ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: icon_small,
        };
        if RegisterClassExW(&wc) == 0 {
            return;
        }

        let scale = system::system_dpi() / 96.0;
        let mut work: RECT = std::mem::zeroed();
        SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut RECT as *mut c_void, 0);
        let work_w = (work.right - work.left).max(800);
        let work_h = (work.bottom - work.top).max(600);
        let win_w = ((1320.0 * scale) as i32)
            .min(work_w - 24)
            .max(940.min(work_w - 24).max(720));
        let win_h = ((860.0 * scale) as i32)
            .min(work_h - 24)
            .max(700.min(work_h - 24).max(520));
        let x = work.left + (work_w - win_w) / 2;
        let y = work.top + (work_h - win_h) / 2;

        let hwnd = CreateWindowExW(
            WS_EX_APPWINDOW,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU | WS_CLIPCHILDREN,
            x,
            y,
            win_w,
            win_h,
            ptr::null_mut(),
            ptr::null_mut(),
            hinstance,
            ptr::null(),
        );
        if hwnd.is_null() {
            return;
        }

        let app = Box::new(App::new(hwnd, hinstance));
        let app_ptr = Box::into_raw(app);
        APP_PTR.store(app_ptr, Ordering::SeqCst);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, app_ptr as isize);
        system::round_window_corners(hwnd);
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
        SetFocus(hwnd);

        let mut msg: MSG = std::mem::zeroed();
        loop {
            let ret = GetMessageW(&mut msg, ptr::null_mut(), 0, 0);
            if ret <= 0 {
                break;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn low_word(v: isize) -> i32 {
    (v as u32 & 0xFFFF) as i16 as i32
}

fn high_word(v: isize) -> i32 {
    ((v as u32 >> 16) & 0xFFFF) as i16 as i32
}

fn high_word_usize(v: usize) -> i32 {
    ((v as u32 >> 16) & 0xFFFF) as i16 as i32
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let app = APP_PTR.load(Ordering::Relaxed);
    if msg == WM_NCCALCSIZE {
        return 0;
    }
    if msg == WM_GETMINMAXINFO {
        let info = &mut *(lparam as *mut MINMAXINFO);
        let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        let mut work_w = 1280;
        let mut work_h = 800;
        if GetMonitorInfoW(mon, &mut mi) != 0 {
            work_w = (mi.rcWork.right - mi.rcWork.left).max(640);
            work_h = (mi.rcWork.bottom - mi.rcWork.top).max(480);
            let (bx, by) = system::resize_border();
            info.ptMaxPosition.x = mi.rcWork.left - mi.rcMonitor.left - bx;
            info.ptMaxPosition.y = mi.rcWork.top - mi.rcMonitor.top - by;
            info.ptMaxSize.x = work_w + bx * 2;
            info.ptMaxSize.y = work_h + by * 2;
        }
        info.ptMinTrackSize.x = 940.min(work_w - 20).max(720);
        info.ptMinTrackSize.y = 680.min(work_h - 20).max(520);
        return 0;
    }
    if msg == WM_NCHITTEST {
        if !app.is_null() {
            let mut pt = POINT {
                x: low_word(lparam),
                y: high_word(lparam),
            };
            ScreenToClient(hwnd, &mut pt);
            return (*app).nonclient_hit_test(pt.x, pt.y);
        }
    }

    if !app.is_null() {
        let app = &mut *app;
        match msg {
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);
                app.paint(hdc);
                EndPaint(hwnd, &ps);
                return 0;
            }
            WM_PASTE => {
                app.paste_clipboard();
                return 0;
            }
            WM_PRINTCLIENT => {
                app.paint(wparam as *mut c_void);
                return 0;
            }
            WM_ERASEBKGND => return 1,
            WM_SIZE => {
                let w = low_word(lparam);
                let h = high_word(lparam);
                app.resize(w, h);
                app.set_maximized(IsZoomed(hwnd) != 0);
                return 0;
            }
            WM_DPICHANGED => {
                let dpi = high_word_usize(wparam) as f32;
                let rect = &*(lparam as *const RECT);
                SetWindowPos(
                    hwnd,
                    ptr::null_mut(),
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    SWP_NOZORDER_NOACTIVATE,
                );
                app.on_dpi_changed(dpi);
                return 0;
            }
            WM_MOUSEMOVE => {
                let mut tme = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                TrackMouseEvent(&mut tme);
                app.mouse_move(low_word(lparam) as f32, high_word(lparam) as f32);
                return 0;
            }
            WM_MOUSELEAVE_MSG => {
                app.mouse_leave();
                return 0;
            }
            WM_LBUTTONDOWN => {
                SetFocus(hwnd);
                SetCapture(hwnd);
                app.mouse_down(low_word(lparam) as f32, high_word(lparam) as f32, 1);
                return 0;
            }
            WM_LBUTTONUP => {
                app.mouse_up();
                ReleaseCapture();
                return 0;
            }
            WM_LBUTTONDBLCLK => {
                app.mouse_double(low_word(lparam) as f32, high_word(lparam) as f32);
                return 0;
            }
            WM_MBUTTONDOWN => {
                SetCapture(hwnd);
                app.mouse_down(low_word(lparam) as f32, high_word(lparam) as f32, 2);
                return 0;
            }
            WM_MBUTTONUP => {
                app.mouse_up();
                ReleaseCapture();
                return 0;
            }
            WM_RBUTTONDOWN => {
                app.right_down(low_word(lparam) as f32, high_word(lparam) as f32);
                return 0;
            }
            WM_RBUTTONUP => {
                return 0;
            }
            WM_MOUSEWHEEL => {
                let delta = (((wparam >> 16) & 0xFFFF) as u16 as i16) as i32;
                let mut pt = POINT {
                    x: low_word(lparam),
                    y: high_word(lparam),
                };
                ScreenToClient(hwnd, &mut pt);
                app.wheel(pt.x as f32, pt.y as f32, delta);
                return 0;
            }
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                let ctrl = GetKeyState(VK_CONTROL as i32) < 0;
                let shift = GetKeyState(VK_SHIFT as i32) < 0;
                if app.key_down(wparam, ctrl, shift) {
                    return 0;
                }
            }
            WM_KEYUP | WM_SYSKEYUP => {
                app.key_up(wparam);
            }
            WM_CHAR => {
                if let Some(ch) = char::from_u32(wparam as u32) {
                    if app.char_input(ch) {
                        return 0;
                    }
                }
            }
            WM_TIMER => {
                app.on_timer();
                return 0;
            }
            WM_SETCURSOR => {
                if (lparam as u32 & 0xFFFF) as u32 == HTCLIENT {
                    let cur = match app.cursor() {
                        Cursor::Arrow => IDC_ARROW,
                        Cursor::Hand => IDC_HAND,
                        Cursor::IBeam => IDC_IBEAM,
                        Cursor::Crosshair => IDC_CROSS,
                        Cursor::SizeNS => IDC_SIZENS,
                        Cursor::SizeWE => IDC_SIZEWE,
                        Cursor::SizeNWSE => IDC_SIZENWSE,
                        Cursor::SizeNESW => IDC_SIZENESW,
                        Cursor::Move => IDC_SIZEALL,
                    };
                    SetCursor(LoadCursorW(ptr::null_mut(), cur));
                    return 1;
                }
            }
            WM_DROPFILES => {
                let drop = wparam as HDROP;
                let count = DragQueryFileW(drop, u32::MAX, ptr::null_mut(), 0);
                let mut paths = Vec::new();
                for i in 0..count {
                    let len = DragQueryFileW(drop, i, ptr::null_mut(), 0);
                    let mut buf = vec![0u16; len as usize + 1];
                    DragQueryFileW(drop, i, buf.as_mut_ptr(), buf.len() as u32);
                    paths.push(std::path::PathBuf::from(String::from_utf16_lossy(
                        &buf[..len as usize],
                    )));
                }
                DragFinish(drop);
                app.on_drop(paths);
                return 0;
            }
            WM_COMMAND => {
                let code = high_word_usize(wparam) as u32;
                let control = lparam as HWND;
                app.command(control, code);
                return 0;
            }
            WM_CTLCOLOREDIT => {
                SetBkColor(wparam as *mut c_void, 0x00FF_FFFF);
                return white_brush() as LRESULT;
            }
            WM_CAPTURECHANGED => {
                app.capture_lost();
                return 0;
            }
            WM_CLOSE => {
                if app.force_close {
                    DestroyWindow(hwnd);
                } else {
                    app.request_close();
                }
                return 0;
            }
            WM_DESTROY => {
                app.on_destroy();
                PostQuitMessage(0);
                return 0;
            }
            WM_NCDESTROY => {
                let ptr = APP_PTR.swap(ptr::null_mut(), Ordering::SeqCst);
                if !ptr.is_null() {
                    drop(Box::from_raw(ptr));
                }
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            _ => {}
        }
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

unsafe extern "system" fn editor_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let app = APP_PTR.load(Ordering::Relaxed);
    if !app.is_null() {
        if msg == WM_KEYDOWN {
            let ctrl = GetKeyState(VK_CONTROL as i32) < 0;
            if (*app).on_editor_key(wparam, ctrl) {
                return 0;
            }
        }
        if msg == WM_CHAR {
            let ch = wparam as u32;
            if ch == 13 && GetKeyState(VK_CONTROL as i32) < 0 {
                return 0;
            }
            if ch == 27 {
                return 0;
            }
        }
    }
    let old = EDIT_OLD_PROC.load(Ordering::Relaxed);
    if old != 0 {
        let proc: windows_sys::Win32::UI::WindowsAndMessaging::WNDPROC =
            std::mem::transmute(old);
        CallWindowProcW(proc, hwnd, msg, wparam, lparam)
    } else {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

pub fn create_edit_control(
    parent: HWND,
    rect: crate::model::Rect,
    font: *mut c_void,
    text: &str,
) -> HWND {
    unsafe {
        let class = system::wide("EDIT");
        let text_w = system::wide(text);
        let hinstance = GetModuleHandleW(ptr::null());
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            text_w.as_ptr(),
            WS_CHILD
                | WS_VISIBLE
                | WS_BORDER
                | WS_TABSTOP
                | ES_MULTILINE as u32
                | ES_AUTOVSCROLL as u32
                | ES_WANTRETURN as u32,
            rect.x.round() as i32,
            rect.y.round() as i32,
            rect.w.round().max(40.0) as i32,
            rect.h.round().max(30.0) as i32,
            parent,
            1001usize as *mut c_void,
            hinstance,
            ptr::null(),
        );
        if hwnd.is_null() {
            return hwnd;
        }
        SendMessageW(hwnd, WM_SETFONT, font as WPARAM, 1);
        let old = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, editor_proc as *const () as isize);
        EDIT_OLD_PROC.store(old, Ordering::SeqCst);
        SendMessageW(hwnd, EM_SETSEL, usize::MAX, -1);
        SetFocus(hwnd);
        hwnd
    }
}

