//! Linux: a fixed-size X11 window (XWayland on Wayland desktops) that shows the frame from
//! `ui` and switches card on a click.

use super::Error;
use crate::ui::{self, Fonts, Frame, View};
use std::borrow::Cow;
use x11rb::connection::Connection;
use x11rb::image::{BitsPerPixel, Image, ImageOrder, ScanlinePad};
use x11rb::properties::WmSizeHints;
use x11rb::protocol::xproto::{
    AtomEnum, ConnectionExt as _, CreateGCAux, CreateWindowAux, EventMask, PropMode, WindowClass,
};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::COPY_DEPTH_FROM_PARENT;

/// Escape, on both the evdev and the old XFree86 keymaps.
const KEY_ESCAPE: u8 = 9;

pub fn run(view: &mut View) -> Result<(), Error> {
    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];
    if screen.root_depth != 24 && screen.root_depth != 32 {
        return Err(format!("unsupported depth {}", screen.root_depth).into());
    }
    let fonts = Fonts::load();
    let height = ui::height_for_all(&fonts, view);
    let (w, h) = (ui::WIDTH as u16, height as u16);

    let window = conn.generate_id()?;
    conn.create_window(
        COPY_DEPTH_FROM_PARENT,
        window,
        screen.root,
        0,
        0,
        w,
        h,
        0,
        WindowClass::INPUT_OUTPUT,
        0,
        &CreateWindowAux::new()
            .background_pixel(screen.white_pixel)
            .event_mask(EventMask::EXPOSURE | EventMask::BUTTON_PRESS | EventMask::KEY_PRESS),
    )?;
    let atom = |name: &[u8]| -> Result<u32, Error> { Ok(conn.intern_atom(false, name)?.reply()?.atom) };
    let (protocols, delete) = (atom(b"WM_PROTOCOLS")?, atom(b"WM_DELETE_WINDOW")?);
    // WM_NAME is Latin-1, which has no "•": window managers read _NET_WM_NAME first.
    conn.change_property8(
        PropMode::REPLACE,
        window,
        atom(b"_NET_WM_NAME")?,
        atom(b"UTF8_STRING")?,
        ui::TITLE.as_bytes(),
    )?;
    conn.change_property8(PropMode::REPLACE, window, AtomEnum::WM_NAME, AtomEnum::STRING, b"WOL-E Link")?;
    conn.change_property8(
        PropMode::REPLACE,
        window,
        AtomEnum::WM_CLASS,
        AtomEnum::STRING,
        b"wole-link\0wole-link\0",
    )?;
    conn.change_property32(PropMode::REPLACE, window, protocols, AtomEnum::ATOM, &[delete])?;
    let mut hints = WmSizeHints::new();
    hints.min_size = Some((w as i32, h as i32));
    hints.max_size = Some((w as i32, h as i32));
    hints.set_normal_hints(&conn, window)?;

    let gc = conn.generate_id()?;
    conn.create_gc(gc, window, &CreateGCAux::new())?;
    conn.map_window(window)?;
    conn.flush()?;

    let mut frame = ui::render(&fonts, view, height);
    loop {
        match conn.wait_for_event()? {
            Event::Expose(e) if e.count == 0 => show(&conn, window, gc, &frame)?,
            Event::ButtonPress(e) if e.detail == 1 => {
                if let Some(i) = frame.hit(e.event_x as i32, e.event_y as i32) {
                    if i != view.chosen {
                        view.chosen = i;
                        frame = ui::render(&fonts, view, height);
                        show(&conn, window, gc, &frame)?;
                    }
                }
            }
            Event::KeyPress(e) if e.detail == KEY_ESCAPE => return Ok(()),
            Event::ClientMessage(e) if e.format == 32 && e.data.as_data32()[0] == delete => {
                return Ok(())
            }
            _ => {}
        }
    }
}

/// Puts the frame on the window; x11rb converts the pixels to the server's format and splits
/// the image into requests under the server's size limit.
fn show(conn: &RustConnection, window: u32, gc: u32, frame: &Frame) -> Result<(), Error> {
    let c = &frame.canvas;
    let bytes: Vec<u8> = c.pixels.iter().flat_map(|p| p.to_le_bytes()).collect();
    let image = Image::new(
        c.width as u16,
        c.height as u16,
        ScanlinePad::Pad32,
        24,
        BitsPerPixel::B32,
        ImageOrder::LsbFirst,
        Cow::Owned(bytes),
    )?;
    image.native(conn.setup())?.put(conn, window, gc, 0, 0)?;
    conn.flush()?;
    Ok(())
}
