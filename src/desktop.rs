//! Linux: an applications-menu entry, only when the user asks for it from the menu. GNOME 45 and
//! later ignore the icon a window carries and show a gear unless a `.desktop` file names the
//! window's class; this writes that file and the icon, and removes exactly those files again.

use crate::i18n::{text, Lang, Msg};
use std::path::{Path, PathBuf};

const ICONS: &[(u32, &[u8])] = &[
    (16, include_bytes!("../assets/icon/wole-link-16.png")),
    (32, include_bytes!("../assets/icon/wole-link-32.png")),
    (48, include_bytes!("../assets/icon/wole-link-48.png")),
    (64, include_bytes!("../assets/icon/wole-link-64.png")),
    (128, include_bytes!("../assets/icon/wole-link-128.png")),
    (256, include_bytes!("../assets/icon/wole-link-256.png")),
];

/// Only Linux desktops read these files.
pub fn available() -> bool {
    cfg!(target_os = "linux")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Missing,
    Installed,
    /// The entry starts a wole-link that isn't this one (the file was moved or replaced).
    Moved,
}

fn data_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
}

fn entry_path(data: &Path) -> PathBuf {
    data.join("applications/wole-link.desktop")
}

fn icon_path(data: &Path, size: u32) -> PathBuf {
    data.join(format!("icons/hicolor/{size}x{size}/apps/wole-link.png"))
}

/// `Exec=` for this executable, quoted and escaped as the Desktop Entry spec asks.
fn exec_line() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    Some(exec_value(&exe.to_string_lossy()))
}

fn exec_value(path: &str) -> String {
    let mut quoted = String::from("\"");
    for ch in path.chars() {
        match ch {
            // Escaped inside the quotes, then the backslash itself escaped for the string value.
            '"' | '`' | '$' | '\\' => {
                quoted.push_str("\\\\");
                quoted.push(ch);
            }
            '%' => quoted.push_str("%%"),
            _ => quoted.push(ch),
        }
    }
    quoted.push('"');
    quoted
}

pub fn state() -> State {
    let (Some(data), Some(exec)) = (data_dir(), exec_line()) else { return State::Missing };
    match std::fs::read_to_string(entry_path(&data)) {
        Err(_) => State::Missing,
        Ok(entry) if entry.lines().any(|l| l.strip_prefix("Exec=") == Some(exec.as_str())) => {
            State::Installed
        }
        Ok(_) => State::Moved,
    }
}

pub fn install() -> std::io::Result<()> {
    let missing = || std::io::Error::other("no home directory");
    let data = data_dir().ok_or_else(missing)?;
    let exec = exec_line().ok_or_else(missing)?;
    for (size, png) in ICONS {
        let path = icon_path(&data, *size);
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, png)?;
    }
    let comment = |lang| text(lang, Msg::AboutText);
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=WOL•E Link\nComment={}\nComment[pt_BR]={}\n\
         Comment[pt_PT]={}\nComment[es]={}\nExec={exec}\nIcon=wole-link\nStartupWMClass=wole-link\n\
         Terminal=false\nCategories=Network;Utility;\n",
        comment(Lang::En),
        comment(Lang::PtBr),
        comment(Lang::PtPt),
        comment(Lang::Es),
    );
    let path = entry_path(&data);
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(path, entry)
}

pub fn remove() -> std::io::Result<()> {
    let data = data_dir().ok_or_else(|| std::io::Error::other("no home directory"))?;
    for (size, _) in ICONS {
        let _ = std::fs::remove_file(icon_path(&data, *size));
    }
    std::fs::remove_file(entry_path(&data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_is_quoted_and_escaped() {
        assert_eq!(exec_value("/home/ana/bin/wole-link"), "\"/home/ana/bin/wole-link\"");
        assert_eq!(exec_value("/media/ana/USB 3/wole-link"), "\"/media/ana/USB 3/wole-link\"");
        assert_eq!(exec_value("/tmp/100%/a$b"), "\"/tmp/100%%/a\\\\$b\"");
    }
}
