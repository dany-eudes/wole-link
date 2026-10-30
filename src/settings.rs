//! The one thing remembered between runs: the language picked in the menu. A small file in
//! `$XDG_CONFIG_HOME/wole-link/` (`~/.config` by default) or `%APPDATA%\wole-link\`, written
//! only when the user picks a language. Nothing breaks when it can't be read or written (a
//! read-only pen drive): the system's language is used.

use crate::i18n::{self, Lang};
use std::path::PathBuf;

const FILE: &str = "settings.ini";

fn dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    base.map(|b| b.join("wole-link"))
}

/// `lang=pt-BR`; anything else in the file is ignored.
pub fn saved_lang() -> Option<Lang> {
    let text = std::fs::read_to_string(dir()?.join(FILE)).ok()?;
    parse_lang(&text)
}

fn parse_lang(text: &str) -> Option<Lang> {
    let tag = text.lines().find_map(|l| l.trim().strip_prefix("lang="))?.trim();
    // A tag we don't know (a newer version's?) is no choice at all.
    Lang::ALL.into_iter().find(|l| l.tag().eq_ignore_ascii_case(tag)).or_else(|| {
        let parsed = i18n::parse(tag);
        (parsed != Lang::En || tag.starts_with("en")).then_some(parsed)
    })
}

#[cfg(test)]
pub fn save_lang(_lang: Lang) {}

#[cfg(not(test))]
pub fn save_lang(lang: Lang) {
    let Some(dir) = dir() else { return };
    let _ = std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(dir.join(FILE), format!("lang={}\n", lang.tag())));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_line() {
        assert_eq!(parse_lang("lang=pt-PT\n"), Some(Lang::PtPt));
        assert_eq!(parse_lang("# hand edited\nlang = es"), None, "the key is `lang=` exactly");
        assert_eq!(parse_lang("other=1\nlang=es\n"), Some(Lang::Es));
        assert_eq!(parse_lang("lang=klingon"), None);
        assert_eq!(parse_lang(""), None);
    }
}
