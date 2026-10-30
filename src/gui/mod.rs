//! The window. When it can't open (no display, an SSH session), `main` falls back to the QR in
//! the terminal.

use crate::ui::View;

pub type Error = Box<dyn std::error::Error>;

#[cfg(all(unix, not(target_os = "macos")))]
mod x11;
#[cfg(windows)]
mod windows;

#[cfg(all(unix, not(target_os = "macos")))]
pub fn run(view: &mut View) -> Result<(), Error> {
    x11::run(view)
}

#[cfg(windows)]
pub fn run(view: &mut View) -> Result<(), Error> {
    windows::run(view)
}

/// A message for when there's nothing to put in the window. Only where a double click leaves
/// no terminal to print to (Windows); elsewhere the terminal says it.
#[cfg(windows)]
pub fn message(text: &str) -> Result<(), Error> {
    windows::message(text)
}

#[cfg(not(windows))]
pub fn message(_text: &str) -> Result<(), Error> {
    Err("printed in the terminal instead".into())
}
