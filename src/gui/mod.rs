//! The window. When it can't open (no display, an SSH session, a Windows build without one
//! yet), `main` falls back to the QR in the terminal.

use crate::ui::View;

pub type Error = Box<dyn std::error::Error>;

#[cfg(all(unix, not(target_os = "macos")))]
mod x11;

#[cfg(all(unix, not(target_os = "macos")))]
pub fn run(view: &mut View) -> Result<(), Error> {
    x11::run(view)
}

#[cfg(not(all(unix, not(target_os = "macos"))))]
pub fn run(_view: &mut View) -> Result<(), Error> {
    Err("no window on this system yet".into())
}
