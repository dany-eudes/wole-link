//! Windows: a fixed-size Win32 window that shows the frame from `ui` and switches card on a
//! click.

use super::Error;
use crate::i18n::Lang;
use crate::net::Card;
use crate::ui::{self, Fonts, Frame, Size, View};
use std::cell::RefCell;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, InvalidateRect, SetDIBitsToDevice, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, PAINTSTRUCT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetMessageW, GetSystemMetrics, LoadCursorW, MessageBoxW, PostQuitMessage, RegisterClassW,
    SystemParametersInfoW, TranslateMessage, CW_USEDEFAULT, IDC_ARROW, MB_ICONWARNING, MB_OK, MSG,
    SM_CYCAPTION, SPI_GETWORKAREA, WM_DESTROY, WM_KEYDOWN, WM_LBUTTONDOWN, WM_PAINT, WNDCLASSW,
    WS_CAPTION, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU,
};

const STYLE: u32 = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;

/// What the window procedure needs; owned, since it lives in a thread-local.
struct State {
    fonts: Fonts,
    lang: Lang,
    cards: Vec<Card>,
    hostname: Option<String>,
    chosen: usize,
    size: Size,
    frame: Frame,
}

impl State {
    fn view(&self) -> View<'_> {
        View { lang: self.lang, cards: &self.cards, chosen: self.chosen, hostname: self.hostname.as_deref() }
    }
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

pub fn message(text: &str) -> Result<(), Error> {
    let (text, title) = (wide(text), wide(ui::TITLE));
    // SAFETY: both strings are NUL-terminated and outlive the call.
    unsafe { MessageBoxW(null_mut(), text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONWARNING) };
    Ok(())
}

pub fn run(view: &mut View) -> Result<(), Error> {
    // The work area leaves out the taskbar; the title bar comes off it too.
    let mut work = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    // SAFETY: SPI_GETWORKAREA writes one RECT.
    let caption = unsafe {
        SystemParametersInfoW(SPI_GETWORKAREA, 0, (&mut work as *mut RECT).cast(), 0);
        GetSystemMetrics(SM_CYCAPTION)
    };
    let max_height = (work.bottom - work.top - caption - 16).max(0) as u32;
    let fonts = Fonts::load();
    let size = ui::fit(&fonts, view, max_height);
    let frame = ui::render(&fonts, view, &size);
    STATE.with(|s| {
        *s.borrow_mut() = Some(State {
            fonts,
            lang: view.lang,
            cards: view.cards.to_vec(),
            hostname: view.hostname.map(str::to_string),
            chosen: view.chosen,
            size,
            frame,
        })
    });

    let class = wide("wole-link");
    let title = wide(ui::TITLE);
    // SAFETY: Win32 window setup; the strings outlive the calls that read them.
    unsafe {
        let instance = GetModuleHandleW(null());
        let mut wc: WNDCLASSW = std::mem::zeroed();
        wc.lpfnWndProc = Some(window_proc);
        wc.hInstance = instance;
        wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
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
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    if let Some(state) = STATE.with(|s| s.borrow_mut().take()) {
        view.chosen = state.chosen;
    }
    Ok(())
}

/// Borrows of STATE never span a call that can come back in here (DestroyWindow sends
/// WM_DESTROY right away), and the state may be missing while the window is being created.
unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            STATE.with(|s| {
                if let Some(state) = s.borrow().as_ref() {
                    let c = &state.frame.canvas;
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
            let changed = STATE.with(|s| {
                let mut s = s.borrow_mut();
                let Some(state) = s.as_mut() else { return false };
                match state.frame.hit(x, y) {
                    Some(i) if i != state.chosen => {
                        state.chosen = i;
                        state.frame = ui::render(&state.fonts, &state.view(), &state.size);
                        true
                    }
                    _ => false,
                }
            });
            if changed {
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_KEYDOWN if wparam == VK_ESCAPE as usize => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
