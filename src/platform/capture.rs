use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use std::time::Duration;

use windows_sys::Win32::Foundation::{
    HANDLE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen,
    CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect,
    GetDC, GetDIBits, GetStockObject, InvalidateRect, ReleaseDC, RoundRect, SelectObject,
    SetBkMode, SetTextColor, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, DT_CENTER,
    DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, HBITMAP, HDC, HGDIOBJ, PAINTSTRUCT, PS_SOLID, SRCCOPY,
};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, SetFocus};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetAncestor, GetClientRect, GetSystemMetrics,
    GetTopWindow, GetWindow, GetWindowRect, IsIconic, IsWindowVisible, LoadCursorW, PostMessageW,
    RegisterClassExW, SetCursor, SetForegroundWindow, ShowWindow, CS_HREDRAW,
    CS_VREDRAW, GA_ROOT, GW_HWNDNEXT, IDC_ARROW, IDC_CROSS, SM_CXSCREEN, SM_CXVIRTUALSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_HIDE, SW_MINIMIZE, SW_RESTORE,
    SW_SHOW, WM_CLOSE, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_PAINT, WM_RBUTTONDOWN, WM_SETCURSOR, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP,
};

use super::system::wide;

const BAR_W: i32 = 480;
const BAR_H: i32 = 64;
const CF_DIB: u32 = 8;
const WM_PASTE: u32 = 0x0302;
const VK_ESCAPE: usize = 0x1B;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Window,
    Rect,
}

struct Desktop {
    mem: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    vx: i32,
    vy: i32,
    w: i32,
    h: i32,
}

impl Drop for Desktop {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.mem, self.old);
            DeleteObject(self.bmp);
            DeleteDC(self.mem);
        }
    }
}

impl Desktop {
    fn bits(&self) -> Option<Vec<u8>> {
        unsafe { get_bits(self.mem, self.bmp, self.old, self.w, self.h) }
    }
}

struct Session {
    main: HWND,
    bar: HWND,
    overlay: HWND,
    mode: Mode,
    desktop: Option<Desktop>,
    start: POINT,
    dragging: bool,
    select: RECT,
    target: HWND,
    target_rect: RECT,
}

struct Captured {
    w: i32,
    h: i32,
    bgra: Vec<u8>,
}

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

static INIT: Once = Once::new();
static CLASSES_OK: AtomicBool = AtomicBool::new(false);

pub fn start(main: HWND) {
    if SESSION.with(|cell| cell.borrow().is_some()) {
        return;
    }
    if !ensure_classes() || main.is_null() {
        return;
    }
    unsafe {
        ShowWindow(main, SW_MINIMIZE);
    }
    let bar = unsafe { create_bar() };
    if bar.is_null() {
        restore_main(main);
        return;
    }
    SESSION.with(|cell| {
        *cell.borrow_mut() = Some(Session {
            main,
            bar,
            overlay: std::ptr::null_mut(),
            mode: Mode::Rect,
            desktop: None,
            start: POINT { x: 0, y: 0 },
            dragging: false,
            select: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            target: std::ptr::null_mut(),
            target_rect: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
        });
    });
    unsafe {
        ShowWindow(bar, SW_SHOW);
        SetForegroundWindow(bar);
        SetFocus(bar);
    }
}

fn ensure_classes() -> bool {
    INIT.call_once(|| {
        let ok = unsafe {
            register_class("FreeCollage.CaptureBar", Some(bar_proc), IDC_ARROW)
                && register_class("FreeCollage.CaptureOverlay", Some(overlay_proc), IDC_CROSS)
        };
        CLASSES_OK.store(ok, Ordering::SeqCst);
    });
    CLASSES_OK.load(Ordering::SeqCst)
}

unsafe fn register_class(
    name: &str,
    proc: windows_sys::Win32::UI::WindowsAndMessaging::WNDPROC,
    cursor: *const u16,
) -> bool {
    let instance = GetModuleHandleW(std::ptr::null());
    let name = wide(name);
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: proc,
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: std::ptr::null_mut(),
        hCursor: LoadCursorW(std::ptr::null_mut(), cursor),
        hbrBackground: GetStockObject(0),
        lpszMenuName: std::ptr::null(),
        lpszClassName: name.as_ptr(),
        hIconSm: std::ptr::null_mut(),
    };
    RegisterClassExW(&wc) != 0
}

unsafe fn create_bar() -> HWND {
    let name = wide("FreeCollage.CaptureBar");
    let instance = GetModuleHandleW(std::ptr::null());
    let screen_w = GetSystemMetrics(SM_CXSCREEN);
    let x = ((screen_w - BAR_W) / 2).max(0);
    CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        name.as_ptr(),
        wide("截图").as_ptr(),
        WS_POPUP,
        x,
        64,
        BAR_W,
        BAR_H,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        instance,
        std::ptr::null(),
    )
}

unsafe fn create_overlay(vx: i32, vy: i32, w: i32, h: i32) -> HWND {
    let name = wide("FreeCollage.CaptureOverlay");
    let instance = GetModuleHandleW(std::ptr::null());
    CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        name.as_ptr(),
        std::ptr::null(),
        WS_POPUP,
        vx,
        vy,
        w,
        h,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        instance,
        std::ptr::null(),
    )
}

unsafe extern "system" fn bar_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            paint_bar(hwnd, hdc);
            EndPaint(hwnd, &ps);
            0
        }
        WM_LBUTTONDOWN => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            bar_click(x);
            0
        }
        WM_KEYDOWN => {
            if wparam == VK_ESCAPE {
                cancel_capture();
            }
            0
        }
        WM_CLOSE => {
            cancel_capture();
            0
        }
        WM_SETCURSOR => {
            SetCursor(LoadCursorW(std::ptr::null_mut(), IDC_ARROW));
            1
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn bar_click(x: i32) {
    if (16..206).contains(&x) {
        begin_mode(Mode::Window);
    } else if (218..408).contains(&x) {
        begin_mode(Mode::Rect);
    } else if (424..468).contains(&x) {
        cancel_capture();
    }
}

fn begin_mode(mode: Mode) {
    let (main, bar) = SESSION.with(|cell| {
        cell.borrow()
            .as_ref()
            .map(|s| (s.main, s.bar))
            .unwrap_or((std::ptr::null_mut(), std::ptr::null_mut()))
    });
    if main.is_null() || bar.is_null() {
        return;
    }
    unsafe {
        ShowWindow(bar, SW_HIDE);
    }
    std::thread::sleep(Duration::from_millis(160));
    let Some(desktop) = (unsafe { capture_screen() }) else {
        cancel_capture();
        return;
    };
    let overlay = unsafe { create_overlay(desktop.vx, desktop.vy, desktop.w, desktop.h) };
    if overlay.is_null() {
        cancel_capture();
        return;
    }
    let stored = SESSION.with(|cell| {
        let mut slot = cell.borrow_mut();
        let Some(session) = slot.as_mut() else {
            return false;
        };
        session.mode = mode;
        session.desktop = Some(desktop);
        session.overlay = overlay;
        session.dragging = false;
        session.select = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        session.target = std::ptr::null_mut();
        session.target_rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        true
    });
    if !stored {
        unsafe {
            DestroyWindow(overlay);
        }
        return;
    }
    unsafe {
        ShowWindow(overlay, SW_SHOW);
        SetForegroundWindow(overlay);
        SetFocus(overlay);
        InvalidateRect(overlay, std::ptr::null(), 0);
    }
}

unsafe extern "system" fn overlay_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            SESSION.with(|cell| {
                let slot = cell.borrow();
                if let Some(session) = slot.as_ref() {
                    if let Some(desktop) = &session.desktop {
                        BitBlt(hdc, 0, 0, desktop.w, desktop.h, desktop.mem, 0, 0, SRCCOPY);
                    }
                    paint_hint(hdc, session);
                    paint_target(hdc, session);
                    if session.dragging || session.select.right > session.select.left {
                        paint_selection(hdc, session.select);
                    }
                }
            });
            EndPaint(hwnd, &ps);
            0
        }
        WM_ERASEBKGND => 1,
        WM_MOUSEMOVE => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            let mode = SESSION.with(|cell| cell.borrow().as_ref().map(|s| s.mode));
            if mode == Some(Mode::Window) {
                update_window_target(x, y);
            } else if mode == Some(Mode::Rect) {
                SESSION.with(|cell| {
                    let mut slot = cell.borrow_mut();
                    if let Some(session) = slot.as_mut() {
                        if session.dragging {
                            session.select = normalize_rect(session.start, POINT { x, y });
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                    }
                });
            }
            0
        }
        WM_LBUTTONDOWN => {
            let x = (lparam & 0xFFFF) as i16 as i32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            let mode = SESSION.with(|cell| cell.borrow().as_ref().map(|s| s.mode));
            if mode == Some(Mode::Window) {
                capture_selected_window(x, y);
            } else if mode == Some(Mode::Rect) {
                SESSION.with(|cell| {
                    if let Some(session) = cell.borrow_mut().as_mut() {
                        session.dragging = true;
                        session.start = POINT { x, y };
                        session.select = RECT {
                            left: x,
                            top: y,
                            right: x,
                            bottom: y,
                        };
                    }
                });
                SetCapture(hwnd);
            }
            0
        }
        WM_LBUTTONUP => {
            let selection = SESSION.with(|cell| {
                let mut slot = cell.borrow_mut();
                if let Some(session) = slot.as_mut() {
                    if session.mode == Mode::Rect && session.dragging {
                        session.dragging = false;
                        return Some(session.select);
                    }
                }
                None
            });
            ReleaseCapture();
            if let Some(rect) = selection {
                if rect.right - rect.left > 2 && rect.bottom - rect.top > 2 {
                    if let Some(captured) = crop_capture(rect) {
                        finish_capture(captured);
                        return 0;
                    }
                }
                SESSION.with(|cell| {
                    if let Some(session) = cell.borrow_mut().as_mut() {
                        session.dragging = false;
                        session.select = RECT {
                            left: 0,
                            top: 0,
                            right: 0,
                            bottom: 0,
                        };
                    }
                });
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
            0
        }
        WM_RBUTTONDOWN => {
            cancel_capture();
            0
        }
        WM_KEYDOWN => {
            if wparam == VK_ESCAPE {
                cancel_capture();
            }
            0
        }
        WM_CLOSE => {
            cancel_capture();
            0
        }
        WM_SETCURSOR => {
            SetCursor(LoadCursorW(std::ptr::null_mut(), IDC_CROSS));
            1
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn window_at(
    vx: i32,
    vy: i32,
    x: i32,
    y: i32,
    main: HWND,
    bar: HWND,
    overlay: HWND,
) -> Option<(HWND, RECT)> {
    unsafe {
        let px = vx + x;
        let py = vy + y;
        let mut hwnd = GetTopWindow(std::ptr::null_mut());
        while !hwnd.is_null() {
            let next = GetWindow(hwnd, GW_HWNDNEXT);
            if hwnd != main
                && hwnd != bar
                && hwnd != overlay
                && IsWindowVisible(hwnd) != 0
                && IsIconic(hwnd) == 0
            {
                let mut rc: RECT = std::mem::zeroed();
                if GetWindowRect(hwnd, &mut rc) != 0
                    && rc.right > rc.left
                    && rc.bottom > rc.top
                    && px >= rc.left
                    && px < rc.right
                    && py >= rc.top
                    && py < rc.bottom
                {
                    let root = GetAncestor(hwnd, GA_ROOT);
                    let root = if root.is_null() { hwnd } else { root };
                    if root != main && root != bar && root != overlay {
                        let mut rr: RECT = std::mem::zeroed();
                        if GetWindowRect(root, &mut rr) != 0 && rr.right > rr.left {
                            return Some((root, rr));
                        }
                    }
                }
            }
            hwnd = next;
        }
        None
    }
}

fn update_window_target(x: i32, y: i32) {
    let Some((main, bar, overlay, vx, vy)) = SESSION.with(|cell| {
        cell.borrow()
            .as_ref()
            .and_then(|s| s.desktop.as_ref().map(|d| (s.main, s.bar, s.overlay, d.vx, d.vy)))
    }) else {
        return;
    };
    let found = window_at(vx, vy, x, y, main, bar, overlay);
    let (target, rect) = found.unwrap_or((
        std::ptr::null_mut(),
        RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
    ));
    let changed = SESSION.with(|cell| {
        let mut slot = cell.borrow_mut();
        let Some(session) = slot.as_mut() else {
            return false;
        };
        let same = session.target == target
            && session.target_rect.left == rect.left
            && session.target_rect.top == rect.top
            && session.target_rect.right == rect.right
            && session.target_rect.bottom == rect.bottom;
        if same {
            return false;
        }
        session.target = target;
        session.target_rect = rect;
        true
    });
    if changed {
        unsafe {
            InvalidateRect(overlay, std::ptr::null(), 0);
        }
    }
}

fn capture_selected_window(x: i32, y: i32) {
    let Some((main, bar, overlay, vx, vy, target)) = SESSION.with(|cell| {
        cell.borrow().as_ref().and_then(|s| {
            s.desktop
                .as_ref()
                .map(|d| (s.main, s.bar, s.overlay, d.vx, d.vy, s.target))
        })
    }) else {
        return;
    };
    let target = if target.is_null() {
        window_at(vx, vy, x, y, main, bar, overlay).map(|(h, _)| h)
    } else {
        Some(target)
    };
    let Some(target) = target else {
        return;
    };
    if target == main || target == bar || target == overlay {
        return;
    }
    unsafe {
        ShowWindow(overlay, SW_HIDE);
    }
    std::thread::sleep(Duration::from_millis(80));
    unsafe {
        SetForegroundWindow(target);
    }
    std::thread::sleep(Duration::from_millis(180));
    match unsafe { capture_window(target) } {
        Some(captured) => finish_capture(captured),
        None => unsafe {
            ShowWindow(overlay, SW_SHOW);
            SetForegroundWindow(overlay);
            SetFocus(overlay);
            InvalidateRect(overlay, std::ptr::null(), 0);
        },
    }
}

fn crop_capture(select: RECT) -> Option<Captured> {
    let (bits, w, h) = SESSION.with(|cell| {
        let slot = cell.borrow();
        let session = slot.as_ref()?;
        let desktop = session.desktop.as_ref()?;
        Some((desktop.bits()?, desktop.w, desktop.h))
    })?;
    let left = select.left.clamp(0, w);
    let top = select.top.clamp(0, h);
    let right = select.right.clamp(0, w);
    let bottom = select.bottom.clamp(0, h);
    let cw = (right - left).max(0);
    let ch = (bottom - top).max(0);
    if cw < 2 || ch < 2 {
        return None;
    }
    let stride = w * 4;
    let mut out = vec![0u8; (cw * ch * 4) as usize];
    for row in 0..ch {
        let src = ((top + row) * stride + left * 4) as usize;
        let dst = (row * cw * 4) as usize;
        out[dst..dst + (cw * 4) as usize]
            .copy_from_slice(&bits[src..src + (cw * 4) as usize]);
    }
    Some(Captured { w: cw, h: ch, bgra: out })
}

unsafe fn capture_screen() -> Option<Desktop> {
    let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
    let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
    let w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
    let h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
    if w < 1 || h < 1 {
        return None;
    }
    let screen = GetDC(std::ptr::null_mut());
    if screen.is_null() {
        return None;
    }
    let mem = CreateCompatibleDC(screen);
    let bmp = CreateCompatibleBitmap(screen, w, h);
    if mem.is_null() || bmp.is_null() {
        if !bmp.is_null() {
            DeleteObject(bmp);
        }
        if !mem.is_null() {
            DeleteDC(mem);
        }
        ReleaseDC(std::ptr::null_mut(), screen);
        return None;
    }
    let old = SelectObject(mem, bmp);
    let ok = BitBlt(mem, 0, 0, w, h, screen, vx, vy, SRCCOPY);
    ReleaseDC(std::ptr::null_mut(), screen);
    if ok == 0 {
        SelectObject(mem, old);
        DeleteObject(bmp);
        DeleteDC(mem);
        return None;
    }
    Some(Desktop {
        mem,
        bmp,
        old,
        vx,
        vy,
        w,
        h,
    })
}

unsafe fn capture_window(hwnd: HWND) -> Option<Captured> {
    let mut rect: RECT = std::mem::zeroed();
    if GetWindowRect(hwnd, &mut rect) == 0 {
        return None;
    }
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;
    if w < 1 || h < 1 {
        return None;
    }
    let screen = GetDC(std::ptr::null_mut());
    if screen.is_null() {
        return None;
    }
    let mem = CreateCompatibleDC(screen);
    let bmp = CreateCompatibleBitmap(screen, w, h);
    if mem.is_null() || bmp.is_null() {
        if !bmp.is_null() {
            DeleteObject(bmp);
        }
        if !mem.is_null() {
            DeleteDC(mem);
        }
        ReleaseDC(std::ptr::null_mut(), screen);
        return None;
    }
    let old = SelectObject(mem, bmp);
    let ok = BitBlt(mem, 0, 0, w, h, screen, rect.left, rect.top, SRCCOPY);
    ReleaseDC(std::ptr::null_mut(), screen);
    if ok == 0 {
        SelectObject(mem, old);
        DeleteObject(bmp);
        DeleteDC(mem);
        return None;
    }
    let bits = get_bits(mem, bmp, old, w, h);
    SelectObject(mem, old);
    DeleteObject(bmp);
    DeleteDC(mem);
    bits.map(|bgra| Captured { w, h, bgra })
}

unsafe fn get_bits(
    mem: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    w: i32,
    h: i32,
) -> Option<Vec<u8>> {
    let mut bmi: BITMAPINFO = std::mem::zeroed();
    bmi.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: w,
        biHeight: -h,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        biSizeImage: (w * h * 4) as u32,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    };
    let mut bits = vec![0u8; (w * h * 4) as usize];
    SelectObject(mem, old);
    let lines = GetDIBits(
        mem,
        bmp,
        0,
        h as u32,
        bits.as_mut_ptr() as *mut c_void,
        &mut bmi,
        DIB_RGB_COLORS,
    );
    SelectObject(mem, bmp);
    if lines == 0 {
        None
    } else {
        Some(bits)
    }
}

fn finish_capture(captured: Captured) {
    let session = SESSION.with(|cell| cell.borrow_mut().take());
    let Some(session) = session else {
        return;
    };
    unsafe {
        if !session.overlay.is_null() {
            DestroyWindow(session.overlay);
        }
        if !session.bar.is_null() {
            DestroyWindow(session.bar);
        }
        ShowWindow(session.main, SW_RESTORE);
        SetForegroundWindow(session.main);
        SetFocus(session.main);
    }
    if set_clipboard(session.main, &captured) {
        unsafe {
            PostMessageW(session.main, WM_PASTE, 0, 0);
        }
    }
}

fn cancel_capture() {
    let session = SESSION.with(|cell| cell.borrow_mut().take());
    let Some(session) = session else {
        return;
    };
    unsafe {
        if !session.overlay.is_null() {
            DestroyWindow(session.overlay);
        }
        if !session.bar.is_null() {
            DestroyWindow(session.bar);
        }
    }
    restore_main(session.main);
}

fn restore_main(main: HWND) {
    if main.is_null() {
        return;
    }
    unsafe {
        ShowWindow(main, SW_RESTORE);
        SetForegroundWindow(main);
        SetFocus(main);
    }
}

fn set_clipboard(main: HWND, captured: &Captured) -> bool {
    let stride = captured.w * 4;
    let total = 40 + stride * captured.h;
    unsafe {
        let handle = GlobalAlloc(0x0042, total as usize);
        if handle.is_null() {
            return false;
        }
        let ptr = GlobalLock(handle) as *mut u8;
        if ptr.is_null() {
            return false;
        }
        let header = BITMAPINFOHEADER {
            biSize: 40,
            biWidth: captured.w,
            biHeight: captured.h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: (stride * captured.h) as u32,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };
        std::ptr::write(ptr as *mut BITMAPINFOHEADER, header);
        let pixels = ptr.add(40);
        for row in 0..captured.h {
            let src = (row * stride) as usize;
            let dst = ((captured.h - 1 - row) * stride) as usize;
            std::ptr::copy_nonoverlapping(
                captured.bgra.as_ptr().add(src),
                pixels.add(dst),
                stride as usize,
            );
            for col in 0..captured.w {
                *pixels.add(dst + (col * 4) as usize + 3) = 255;
            }
        }
        GlobalUnlock(handle);
        let mut opened = false;
        for _ in 0..6 {
            if OpenClipboard(main) != 0 {
                opened = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(12));
        }
        if !opened {
            return false;
        }
        EmptyClipboard();
        let ok = SetClipboardData(CF_DIB, handle as HANDLE);
        CloseClipboard();
        if ok.is_null() {
            return false;
        }
        true
    }
}

fn normalize_rect(a: POINT, b: POINT) -> RECT {
    RECT {
        left: a.x.min(b.x),
        top: a.y.min(b.y),
        right: a.x.max(b.x),
        bottom: a.y.max(b.y),
    }
}

fn paint_bar(hwnd: HWND, hdc: HDC) {
    unsafe {
        let mut rc: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut rc);
        let bg = CreateSolidBrush(rgb(255, 255, 255));
        FillRect(hdc, &rc, bg);
        DeleteObject(bg);
        let border_brush = CreateSolidBrush(rgb(100, 116, 139));
        FillRect(
            hdc,
            &RECT { left: 0, top: 0, right: rc.right, bottom: 2 },
            border_brush,
        );
        FillRect(
            hdc,
            &RECT { left: 0, top: rc.bottom - 2, right: rc.right, bottom: rc.bottom },
            border_brush,
        );
        FillRect(
            hdc,
            &RECT { left: 0, top: 0, right: 2, bottom: rc.bottom },
            border_brush,
        );
        FillRect(
            hdc,
            &RECT { left: rc.right - 2, top: 0, right: rc.right, bottom: rc.bottom },
            border_brush,
        );
        DeleteObject(border_brush);

        let font = bar_font(16);
        let old_font = SelectObject(hdc, font);
        SetBkMode(hdc, 1);
        bar_button(
            hdc,
            RECT { left: 16, top: 10, right: 206, bottom: 54 },
            "截图窗口",
            font,
        );
        bar_button(
            hdc,
            RECT { left: 218, top: 10, right: 408, bottom: 54 },
            "矩形选择",
            font,
        );
        bar_button(
            hdc,
            RECT { left: 424, top: 10, right: 468, bottom: 54 },
            "×",
            font,
        );
        SelectObject(hdc, old_font);
        DeleteObject(font);
    }
}

unsafe fn bar_button(hdc: HDC, rect: RECT, label: &str, font: windows_sys::Win32::Graphics::Gdi::HFONT) {
    let brush = CreateSolidBrush(rgb(247, 249, 252));
    let pen = CreatePen(PS_SOLID, 1, rgb(221, 227, 235));
    let old_brush = SelectObject(hdc, brush);
    let old_pen = SelectObject(hdc, pen);
    RoundRect(hdc, rect.left, rect.top, rect.right, rect.bottom, 16, 16);
    SelectObject(hdc, old_brush);
    SelectObject(hdc, old_pen);
    DeleteObject(brush);
    DeleteObject(pen);
    SetTextColor(hdc, rgb(17, 24, 39));
    let old_font = SelectObject(hdc, font);
    text(hdc, label, rect);
    SelectObject(hdc, old_font);
}

fn paint_hint(hdc: HDC, session: &Session) {
    unsafe {
        let Some(desktop) = &session.desktop else {
            return;
        };
        let label = if session.mode == Mode::Rect {
            "拖动选择区域，Esc 取消"
        } else {
            "点击要截图的窗口，Esc 取消"
        };
        let w = 320;
        let left = (desktop.w - w) / 2;
        let rect = RECT {
            left,
            top: 18,
            right: left + w,
            bottom: 58,
        };
        let brush = CreateSolidBrush(rgb(255, 255, 255));
        let pen = CreatePen(PS_SOLID, 1, rgb(210, 216, 224));
        let old_brush = SelectObject(hdc, brush);
        let old_pen = SelectObject(hdc, pen);
        RoundRect(hdc, rect.left, rect.top, rect.right, rect.bottom, 12, 12);
        SelectObject(hdc, old_brush);
        SelectObject(hdc, old_pen);
        DeleteObject(brush);
        DeleteObject(pen);
        let font = bar_font(14);
        let old_font = SelectObject(hdc, font);
        SetBkMode(hdc, 1);
        SetTextColor(hdc, rgb(31, 41, 55));
        text(hdc, label, rect);
        SelectObject(hdc, old_font);
        DeleteObject(font);
    }
}

fn paint_target(hdc: HDC, session: &Session) {
    if session.mode != Mode::Window || session.target.is_null() {
        return;
    }
    let Some(desktop) = &session.desktop else {
        return;
    };
    let left = session.target_rect.left - desktop.vx;
    let top = session.target_rect.top - desktop.vy;
    let right = session.target_rect.right - desktop.vx;
    let bottom = session.target_rect.bottom - desktop.vy;
    if right <= left || bottom <= top {
        return;
    }
    unsafe {
        let pen = CreatePen(PS_SOLID, 4, rgb(37, 99, 235));
        let brush = GetStockObject(5);
        let old_pen = SelectObject(hdc, pen);
        let old_brush = SelectObject(hdc, brush);
        windows_sys::Win32::Graphics::Gdi::Rectangle(hdc, left, top, right, bottom);
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
        DeleteObject(pen);
    }
}

fn paint_selection(hdc: HDC, rect: RECT) {
    unsafe {
        let brush = GetStockObject(5);
        let pen = CreatePen(PS_SOLID, 2, rgb(37, 99, 235));
        let old_brush = SelectObject(hdc, brush);
        let old_pen = SelectObject(hdc, pen);
        windows_sys::Win32::Graphics::Gdi::Rectangle(
            hdc,
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
        );
        SelectObject(hdc, old_brush);
        SelectObject(hdc, old_pen);
        DeleteObject(pen);
    }
}

fn bar_font(size: i32) -> windows_sys::Win32::Graphics::Gdi::HFONT {
    let face = wide("Microsoft YaHei UI");
    unsafe {
        CreateFontW(
            -size,
            0,
            0,
            0,
            500,
            0,
            0,
            0,
            1,
            0,
            3,
            5,
            0,
            face.as_ptr(),
        )
    }
}

fn text(hdc: HDC, value: &str, mut rect: RECT) {
    let wide_value: Vec<u16> = value.encode_utf16().collect();
    unsafe {
        DrawTextW(
            hdc,
            wide_value.as_ptr(),
            wide_value.len() as i32,
            &mut rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
    }
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    r as u32 | ((g as u32) << 8) | ((b as u32) << 16)
}
