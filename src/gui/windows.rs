//! Windows: a fixed-size Win32 window that shows the frame from `ui` and switches card on a
//! click.

use super::Error;
use crate::ui::{self, App, Effect, Start};
use std::cell::RefCell;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, InvalidateRect, SetDIBitsToDevice, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, PAINTSTRUCT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{
    GetDpiForSystem, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_SYSTEM_AWARE,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetMessageW, GetSystemMetrics, LoadCursorW, LoadIconW, LoadImageW, MessageBoxW, SendMessageW,
    ICON_SMALL, IMAGE_ICON, LR_DEFAULTCOLOR, SM_CXSMICON, SM_CYSMICON, WM_SETICON, PostQuitMessage, RegisterClassW,
    SystemParametersInfoW, TranslateMessage, CW_USEDEFAULT, IDC_ARROW, MB_ICONWARNING, MB_OK, MSG,
    SM_CYCAPTION, SPI_GETWORKAREA, WM_DESTROY, WM_KEYDOWN, WM_LBUTTONDOWN, WM_PAINT, WNDCLASSW,
    WS_CAPTION, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU,
};

const ICON_ID: *const u16 = 1 as *const u16;
const STYLE: u32 = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;

thread_local! {
    static STATE: RefCell<Option<App>> = const { RefCell::new(None) };
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

pub fn message(text: &str) -> Result<(), Error> {
    dpi_aware();
    let (text, title) = (wide(text), wide(ui::TITLE));
    // SAFETY: both strings are NUL-terminated and outlive the call.
    unsafe { MessageBoxW(null_mut(), text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONWARNING) };
    Ok(())
}

/// Draws at the system's DPI instead of letting Windows stretch a 96-dpi bitmap (blurry text,
/// QR modules on fractional pixels). Before any window, the message box included.
fn dpi_aware() {
    // SAFETY: a process-wide setting with no pointers; fails harmlessly if already set.
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE) };
}

fn system_scale() -> f32 {
    // SAFETY: no arguments.
    unsafe { GetDpiForSystem() as f32 / 96.0 }
}

pub fn run(start: Start) -> Result<(), Error> {
    dpi_aware();
    let scale = super::scale_override().unwrap_or_else(system_scale);
    // The work area leaves out the taskbar; the title bar comes off it too.
    let mut work = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    // SAFETY: SPI_GETWORKAREA writes one RECT.
    let caption = unsafe {
        SystemParametersInfoW(SPI_GETWORKAREA, 0, (&mut work as *mut RECT).cast(), 0);
        GetSystemMetrics(SM_CYCAPTION)
    };
    let max_height = (work.bottom - work.top - caption - (16.0 * scale) as i32).max(0) as u32;
    let app = App::new(start, max_height, scale);
    let size = app.size();
    STATE.with(|s| *s.borrow_mut() = Some(app));

    let class = wide("wole-link");
    let title = wide(ui::TITLE);
    // SAFETY: Win32 window setup; the strings outlive the calls that read them.
    unsafe {
        let instance = GetModuleHandleW(null());
        let mut wc: WNDCLASSW = std::mem::zeroed();
        wc.lpfnWndProc = Some(window_proc);
        wc.hInstance = instance;
        wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
        // Resource 1, put there by build.rs.
        wc.hIcon = LoadIconW(instance, ICON_ID);
        wc.lpszClassName = class.as_ptr();
        if RegisterClassW(&wc) == 0 {
            return Err("RegisterClassW failed".into());
        }
        let mut outer = RECT { left: 0, top: 0, right: size.width as i32, bottom: size.height as i32 };
        AdjustWindowRectEx(&mut outer, STYLE, 0, 0);
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            STYLE | windows_sys::Win32::UI::WindowsAndMessaging::WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            outer.right - outer.left,
            outer.bottom - outer.top,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );
        if hwnd.is_null() {
            return Err("CreateWindowExW failed".into());
        }
        // The title bar's icon, from the 16/32 px images drawn for that size rather than the
        // big one shrunk.
        let small = LoadImageW(
            instance,
            ICON_ID,
            IMAGE_ICON,
            GetSystemMetrics(SM_CXSMICON),
            GetSystemMetrics(SM_CYSMICON),
            LR_DEFAULTCOLOR,
        );
        if !small.is_null() {
            SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, small as isize);
        }
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    STATE.with(|s| s.borrow_mut().take());
    Ok(())
}

/// Carries out what a click asked for, with STATE no longer borrowed: opening a link and
/// destroying the window can both come back into the window procedure.
unsafe fn apply(hwnd: HWND, effect: Effect) {
    match effect {
        Effect::None => {}
        Effect::Redraw => {
            InvalidateRect(hwnd, null(), 0);
        }
        Effect::Open(url) => {
            InvalidateRect(hwnd, null(), 0);
            crate::open::open(&url);
        }
        Effect::Close => {
            DestroyWindow(hwnd);
        }
    }
}

/// Borrows of STATE never span a call that can come back in here (DestroyWindow sends
/// WM_DESTROY right away), and the state may be missing while the window is being created.
unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            STATE.with(|s| {
                if let Some(app) = s.borrow().as_ref() {
                    let c = &app.frame().canvas;
                    let mut info: BITMAPINFO = std::mem::zeroed();
                    info.bmiHeader = BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: c.width as i32,
                        // Negative: rows go top to bottom, as in the canvas.
                        biHeight: -(c.height as i32),
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB,
                        ..std::mem::zeroed()
                    };
                    // 0x00RRGGBB in little-endian memory is the BGRX that BI_RGB expects.
                    SetDIBitsToDevice(
                        hdc,
                        0,
                        0,
                        c.width,
                        c.height,
                        0,
                        0,
                        0,
                        c.height,
                        c.pixels.as_ptr().cast(),
                        &info,
                        DIB_RGB_COLORS,
                    );
                }
            });
            EndPaint(hwnd, &ps);
            0
        }
        WM_LBUTTONDOWN => {
            let (x, y) = ((lparam & 0xFFFF) as i16 as i32, ((lparam >> 16) & 0xFFFF) as i16 as i32);
            let effect = STATE.with(|s| s.borrow_mut().as_mut().map_or(Effect::None, |app| app.click(x, y)));
            apply(hwnd, effect);
            0
        }
        WM_KEYDOWN if wparam == VK_ESCAPE as usize => {
            let effect = STATE.with(|s| s.borrow_mut().as_mut().map_or(Effect::None, |app| app.escape()));
            apply(hwnd, effect);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
