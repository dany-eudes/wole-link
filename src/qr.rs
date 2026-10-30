//! The QR code drawn in the terminal with half blocks, two modules per character cell.

use qrcodegen::{QrCode, QrCodeEcc};

/// Light margin around the code, in modules. The QR spec asks for 4.
const QUIET: i32 = 4;

/// With `ansi`, the code is painted black on white, whatever the terminal's theme (phone
/// scanners often miss a light-on-dark code). Without it, dark modules are drawn as blocks and
/// the background is assumed light.
pub fn render(text: &str, ansi: bool) -> String {
    let Ok(qr) = QrCode::encode_text(text, QrCodeEcc::Medium) else { return String::new() };
    let size = qr.size();
    let dark = |x: i32, y: i32| qr.get_module(x, y); // false outside the code
    let mut out = String::new();
    let mut y = -QUIET;
    while y < size + QUIET {
        if ansi {
            out.push_str("\x1b[30;47m");
        }
        for x in -QUIET..size + QUIET {
            out.push(match (dark(x, y), dark(x, y + 1)) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                (false, false) => ' ',
            });
        }
        if ansi {
            out.push_str("\x1b[0m");
        }
        out.push('\n');
        y += 2;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_with_quiet_zone() {
        let link = "https://wole.celerasoft.com/add#v=1&mac=AA-BB-CC-0D-EE-01";
        let qr = QrCode::encode_text(link, QrCodeEcc::Medium).unwrap();
        let width = (qr.size() + 2 * QUIET) as usize;
        let drawn = render(link, false);
        let lines: Vec<&str> = drawn.lines().collect();
        assert_eq!(lines.len(), width.div_ceil(2));
        assert!(lines.iter().all(|l| l.chars().count() == width));
        // The top margin row is blank; the finder pattern's top edge comes right after.
        assert!(lines[0].trim().is_empty());
        assert!(lines[2].contains('▀') || lines[2].contains('█'));
    }
}
