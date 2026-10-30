//! The window's contents, drawn into a pixel buffer. The same code serves X11 and Win32: each
//! only shows the buffer and passes clicks back.

use crate::i18n::{text, Lang, Msg};
use crate::link;
use crate::net::{Card, Kind};
use crate::wordmark;
use fontdue::{Font, FontSettings};
use qrcodegen::{QrCode, QrCodeEcc};

const REGULAR: &[u8] = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../assets/fonts/NotoSans-Bold.ttf");

pub const TITLE: &str = "WOL•E Link";
/// The bottom of the wordmark's letters, in its own units (logo_wordmark.xml).
const WORDMARK_BASELINE: f32 = 112.55;
const PAD: i32 = 24;
const GAP: i32 = 24;
/// The right column: the link's data and the cards. The QR takes the left one.
const RIGHT: i32 = 360;
/// Pixels per QR module at scale 1: the largest that keeps the window on the screen, down to the
/// smallest a phone still reads comfortably off a screen (in physical pixels, at any scale).
const MODULE_MAX: f32 = 7.0;
const MODULE_MIN: i32 = 3;
const QR_ECC: QrCodeEcc = QrCodeEcc::Medium;

// The app's light palette (ui/theme/Color.kt), as 0xRRGGBB.
const BACKGROUND: u32 = 0xF4F6F9;
const SURFACE: u32 = 0xFFFFFF;
const BORDER: u32 = 0xCBD5E1;
const TEXT: u32 = 0x0D141C;
const TEXT_SECONDARY: u32 = 0x4F6174;
const ACCENT: u32 = 0x0E84A6;
const SELECTED: u32 = 0xCDE8F2;
const DISABLED: u32 = 0x94A3B8;
const WARNING_BACKGROUND: u32 = 0xFFF4E0;
const WARNING_TEXT: u32 = 0x6B4500;
const QR_DARK: u32 = 0x000000;

pub struct Fonts {
    regular: Font,
    bold: Font,
}

impl Fonts {
    pub fn load() -> Fonts {
        let load = |bytes| Font::from_bytes(bytes, FontSettings::default()).expect("embedded font");
        Fonts { regular: load(REGULAR), bold: load(BOLD) }
    }
}

/// Pixels as 0x00RRGGBB, row by row.
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

pub struct Frame {
    pub canvas: Canvas,
    /// Clickable card rows and the index of their card.
    rows: Vec<(Rect, usize)>,
}

impl Frame {
    pub fn hit(&self, x: i32, y: i32) -> Option<usize> {
        self.rows.iter().find(|(r, _)| r.contains(x, y)).map(|(_, i)| *i)
    }
}

pub struct View<'a> {
    pub lang: Lang,
    pub cards: &'a [Card],
    /// Index into `cards`; must be a usable card.
    pub chosen: usize,
    pub hostname: Option<&'a str>,
}

/// What goes into the link for `card`.
pub fn device(card: &Card, hostname: Option<&str>) -> link::Device {
    let (ip, prefix) = card.ipv4.expect("usable cards have IPv4");
    link::Device {
        mac: card.mac.expect("usable cards have a MAC"),
        name: hostname.map(str::to_string),
        ip: Some(ip),
        broadcast: Some(link::broadcast(ip, prefix)),
        port: Some(9),
    }
}

/// The window's size and the QR's scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
    /// Physical pixels per logical one: 1.5 at 144 dpi or a 150 % Windows setting.
    scale: f32,
    module: i32,
    qr_side: i32,
}

/// The largest QR that keeps the window within `max_height`, everything drawn at `scale`; both in
/// physical pixels. The height is the tallest view among the cards the user can click, so the
/// window doesn't change size when the Wi-Fi warning comes and goes.
pub fn fit(fonts: &Fonts, view: &View, max_height: u32, scale: f32) -> Size {
    let scale = scale.clamp(1.0, 4.0);
    let px = |v: i32| (v as f32 * scale).round() as i32;
    let usable: Vec<usize> = (0..view.cards.len()).filter(|&i| view.cards[i].usable()).collect();
    let modules = usable
        .iter()
        .filter_map(|&i| QrCode::encode_text(&link::build(&device(&view.cards[i], view.hostname)), QR_ECC).ok())
        .map(|qr| qr.size() + 8)
        .max()
        .unwrap_or(0);
    let module_max = (MODULE_MAX * scale).round() as i32;
    let mut size = Size { width: 0, height: 0, scale, module: module_max, qr_side: 0 };
    for module in (MODULE_MIN..=module_max).rev() {
        let qr_side = modules * module;
        let width = (2 * px(PAD) + qr_side + px(GAP) + px(RIGHT)) as u32;
        size = Size { width, height: 0, scale, module, qr_side };
        size.height = usable
            .iter()
            .map(|&i| layout(fonts, &View { chosen: i, ..*view }, &size, None))
            .max()
            .unwrap_or(0) as u32;
        if size.height <= max_height {
            break;
        }
    }
    size
}

pub fn render(fonts: &Fonts, view: &View, size: &Size) -> Frame {
    let mut canvas = Canvas::new(size.width, size.height, BACKGROUND);
    let mut rows = Vec::new();
    layout(fonts, view, size, Some((&mut canvas, &mut rows)));
    Frame { canvas, rows }
}

/// Title on top; QR on the left, data and cards on the right; the link at the bottom. Returns
/// the height, and draws too when given a canvas.
fn layout(
    fonts: &Fonts,
    view: &View,
    size: &Size,
    mut target: Option<(&mut Canvas, &mut Vec<(Rect, usize)>)>,
) -> i32 {
    let lang = view.lang;
    let card = &view.cards[view.chosen];
    let url = link::build(&device(card, view.hostname));
    // Every length below is in logical pixels, scaled here; font sizes too.
    let sc = size.scale;
    let px = |v: i32| (v as f32 * sc).round() as i32;
    let (pad, right) = (px(PAD), px(RIGHT));
    let inner = size.width as i32 - 2 * pad;
    let mut y = pad;

    macro_rules! draw {
        (|$c:ident| $body:expr) => {
            if let Some((ref mut canvas, _)) = target {
                let $c: &mut Canvas = canvas;
                $body;
            }
        };
    }

    // The WOL•E wordmark, then "Link" on the wordmark's baseline.
    let mark_height = 34.0 * sc;
    let k = mark_height / wordmark::HEIGHT;
    let mark_width = (wordmark::WIDTH * k).ceil() as i32;
    let title = Text::new(&fonts.bold, 25.0 * sc, TEXT);
    let baseline = y as f32 + WORDMARK_BASELINE * k;
    draw!(|c| {
        for (color, polygons) in wordmark::SHAPES {
            c.fill(polygons, pad as f32, y as f32, k, *color);
        }
        c.text(&title, pad + mark_width + px(10), baseline.round() as i32 - title.ascent(), "Link");
    });
    y += mark_height.ceil() as i32 + px(6);
    let body = Text::new(&fonts.regular, 15.0 * sc, TEXT_SECONDARY);
    for line in body.wrap(&text(lang, Msg::Scan), inner) {
        draw!(|c| c.text(&body, pad, y, &line));
        y += body.line_height();
    }
    y += px(16);
    let top = y;

    // Left: the QR on a white panel; its quiet zone is part of the panel.
    if let Ok(qr) = QrCode::encode_text(&url, QR_ECC) {
        let (module, side) = (size.module, size.qr_side);
        // A shorter link can make a smaller code than the one the window was sized for.
        let offset = (side - (qr.size() + 8) * module) / 2;
        draw!(|c| {
            c.rect(pad, top, side, side, SURFACE);
            c.outline(pad, top, side, side, BORDER);
            for my in 0..qr.size() {
                for mx in 0..qr.size() {
                    if qr.get_module(mx, my) {
                        let (mx, my) = ((mx + 4) * module + offset, (my + 4) * module + offset);
                        c.rect(pad + mx, top + my, module, module, QR_DARK);
                    }
                }
            }
        });
    }
    let left_bottom = top + size.qr_side;

    // Right: the data in the link, with the app's form labels.
    let x = pad + size.qr_side + px(GAP);
    let label = Text::new(&fonts.regular, 13.0 * sc, TEXT_SECONDARY);
    let value = Text::new(&fonts.bold, 14.0 * sc, TEXT);
    let d = device(card, view.hostname);
    let fields: Vec<(String, String)> = [
        (Msg::LabelName, d.name.clone()),
        (Msg::LabelMac, Some(link::format_mac(&d.mac))),
        (Msg::LabelIp, d.ip.map(|ip| ip.to_string())),
        (Msg::LabelBroadcast, d.broadcast.map(|bc| bc.to_string())),
        (Msg::LabelPort, d.port.map(|p| p.to_string())),
    ]
    .into_iter()
    .filter_map(|(msg, v)| v.map(|v| (text(lang, msg), v)))
    .collect();
    let label_width = fields.iter().map(|(l, _)| label.width(l)).max().unwrap_or(0) + px(16);
    let mut y = top;
    for (l, v) in &fields {
        draw!(|c| {
            // Labels are smaller: line their baseline up with the value's.
            c.text(&label, x, y + value.ascent() - label.ascent(), l);
            c.text(&value, x + label_width, y, v);
        });
        y += value.line_height() + px(2);
    }

    // Wake-on-LAN over Wi-Fi rarely works: say so whenever a Wi-Fi card is chosen.
    if card.kind == Kind::Wireless {
        y += px(10);
        let warning = Text::new(&fonts.regular, 13.0 * sc, WARNING_TEXT);
        let wired_too = view.cards.iter().any(|c| c.kind == Kind::Wired && c.usable());
        let msg = if wired_too { Msg::WirelessChosen } else { Msg::WirelessWarning };
        let lines = warning.wrap(&text(lang, msg), right - px(24));
        let h = lines.len() as i32 * warning.line_height() + px(20);
        draw!(|c| {
            c.rect(x, y, right, h, WARNING_BACKGROUND);
            for (i, line) in lines.iter().enumerate() {
                c.text(&warning, x + px(12), y + px(10) + i as i32 * warning.line_height(), line);
            }
        });
        y += h;
    }
    y += px(18);

    // The physical cards; virtual ones only show in --list.
    let header = Text::new(&fonts.bold, 15.0 * sc, TEXT);
    draw!(|c| c.text(&header, x, y, &text(lang, Msg::Cards)));
    y += header.line_height() + px(6);
    let suggested = crate::net::suggest(view.cards);
    for (i, card) in view.cards.iter().enumerate() {
        if !matches!(card.kind, Kind::Wired | Kind::Wireless) {
            continue;
        }
        let usable = card.usable();
        let chosen = i == view.chosen;
        let (fg, fg2) = if usable { (TEXT, TEXT_SECONDARY) } else { (DISABLED, DISABLED) };
        let name = Text::new(&fonts.bold, 14.0 * sc, fg);
        let detail = Text::new(&fonts.regular, 12.5 * sc, fg2);
        let tag = Text::new(&fonts.regular, 12.5 * sc, ACCENT);
        let h = name.line_height() + detail.line_height() + px(16);
        let rect = Rect { x, y, w: right, h };
        let kind = text(lang, if card.kind == Kind::Wired { Msg::Wired } else { Msg::Wireless });
        let mut second = card.mac.map(|m| link::format_mac(&m)).unwrap_or_default();
        let state = match (card.up, card.ipv4) {
            (false, _) => text(lang, Msg::Disconnected),
            (true, Some((ip, prefix))) => format!("{ip}/{prefix}"),
            (true, None) => text(lang, Msg::NoIpv4),
        };
        second = if second.is_empty() { state } else { format!("{second}  ·  {state}") };
        if let Some((canvas, rows)) = target.as_mut() {
            let c: &mut Canvas = canvas;
            c.rect(rect.x, rect.y, rect.w, rect.h, if chosen { SELECTED } else { SURFACE });
            c.outline(rect.x, rect.y, rect.w, rect.h, if chosen { ACCENT } else { BORDER });
            let (cx, cy) = (rect.x + px(20), rect.y + h / 2);
            c.ring(cx, cy, 7.0 * sc, 1.6 * sc, if usable { ACCENT } else { DISABLED });
            if chosen {
                c.disc(cx, cy, 3.6 * sc, ACCENT);
            }
            let tx = rect.x + px(40);
            // Windows names its cards "Ethernet" and "Wi-Fi": no "Wi-Fi [Wi-Fi]".
            let first = if card.name.to_lowercase().starts_with(&kind.to_lowercase()) {
                card.name.clone()
            } else {
                format!("{}  [{kind}]", card.name)
            };
            c.text(&name, tx, rect.y + px(8), &first);
            if Some(i) == suggested {
                let after = tx + name.width(&first) + px(8);
                c.text(&tag, after, rect.y + px(8) + name.ascent() - tag.ascent(), &text(lang, Msg::Suggested));
            }
            c.text(&detail, tx, rect.y + px(8) + name.line_height(), &second);
            if usable {
                rows.push((rect, i));
            }
        }
        y += h + px(8);
    }
    let mut y = y.max(left_bottom) + px(14);

    // The link itself, for reading it off the screen or comparing with what the phone read.
    let small = Text::new(&fonts.regular, 11.0 * sc, TEXT_SECONDARY);
    for line in small.wrap(&url, inner) {
        draw!(|c| c.text(&small, pad, y, &line));
        y += small.line_height();
    }
    y + pad
}

/// A font at a size and color.
struct Text<'a> {
    font: &'a Font,
    size: f32,
    color: u32,
}

impl<'a> Text<'a> {
    fn new(font: &'a Font, size: f32, color: u32) -> Self {
        Text { font, size, color }
    }

    fn ascent(&self) -> i32 {
        self.font.horizontal_line_metrics(self.size).map_or(self.size, |m| m.ascent).round() as i32
    }

    fn line_height(&self) -> i32 {
        self.font.horizontal_line_metrics(self.size).map_or(self.size * 1.4, |m| m.new_line_size).ceil()
            as i32
    }

    fn width(&self, s: &str) -> i32 {
        let mut width = 0.0;
        let mut prev = None;
        for ch in s.chars() {
            if let Some(p) = prev {
                width += self.font.horizontal_kern(p, ch, self.size).unwrap_or(0.0);
            }
            width += self.font.metrics(ch, self.size).advance_width;
            prev = Some(ch);
        }
        width.ceil() as i32
    }

    /// Breaks at spaces; a word wider than the line (the link) breaks anywhere.
    fn wrap(&self, s: &str, max: i32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut line = String::new();
        for word in s.split(' ') {
            let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if self.width(&candidate) <= max {
                line = candidate;
                continue;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            for ch in word.chars() {
                line.push(ch);
                if self.width(&line) > max {
                    line.pop();
                    lines.push(std::mem::take(&mut line));
                    line.push(ch);
                }
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
        lines
    }
}

impl Canvas {
    fn new(width: u32, height: u32, color: u32) -> Canvas {
        Canvas { width, height, pixels: vec![color; (width * height) as usize] }
    }

    fn blend(&mut self, x: i32, y: i32, color: u32, alpha: f32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 || alpha <= 0.0 {
            return;
        }
        let i = (y as u32 * self.width + x as u32) as usize;
        let a = alpha.min(1.0);
        let mix = |shift: u32| {
            let (s, d) = ((color >> shift) & 0xFF, (self.pixels[i] >> shift) & 0xFF);
            ((s as f32 * a + d as f32 * (1.0 - a)).round() as u32) << shift
        };
        self.pixels[i] = mix(16) | mix(8) | mix(0);
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        for py in y.max(0)..(y + h).min(self.height as i32) {
            for px in x.max(0)..(x + w).min(self.width as i32) {
                self.pixels[(py as u32 * self.width + px as u32) as usize] = color;
            }
        }
    }

    fn outline(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        self.rect(x, y, w, 1, color);
        self.rect(x, y + h - 1, w, 1, color);
        self.rect(x, y, 1, h, color);
        self.rect(x + w - 1, y, 1, h, color);
    }

    /// Anti-aliased by distance from the edge.
    fn disc(&mut self, cx: i32, cy: i32, r: f32, color: u32) {
        self.circle(cx, cy, r, |d| r + 0.5 - d, color);
    }

    fn ring(&mut self, cx: i32, cy: i32, r: f32, thickness: f32, color: u32) {
        self.circle(cx, cy, r, |d| thickness / 2.0 + 0.5 - (d - (r - thickness / 2.0)).abs(), color);
    }

    fn circle(&mut self, cx: i32, cy: i32, r: f32, coverage: impl Fn(f32) -> f32, color: u32) {
        let reach = r.ceil() as i32 + 1;
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let d = ((dx * dx + dy * dy) as f32).sqrt();
                self.blend(cx + dx, cy + dy, color, coverage(d).clamp(0.0, 1.0));
            }
        }
    }

    /// Fills polygons given in `k`-scaled units from (`ox`, `oy`), nonzero rule. Four sample rows
    /// per pixel; across a row, each pixel gets the exact share of the span that covers it.
    fn fill(&mut self, polygons: &[&[(f32, f32)]], ox: f32, oy: f32, k: f32, color: u32) {
        const SUB: usize = 4;
        let map = |&(x, y): &(f32, f32)| (ox + x * k, oy + y * k);
        let points = || polygons.iter().flat_map(|p| p.iter()).map(map);
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for (x, y) in points() {
            (min_x, min_y, max_x, max_y) = (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y));
        }
        let x0 = (min_x.floor() as i32).max(0);
        let x1 = (max_x.ceil() as i32).min(self.width as i32);
        let y0 = (min_y.floor() as i32).max(0);
        let y1 = (max_y.ceil() as i32).min(self.height as i32);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let mut coverage = vec![0f32; (x1 - x0) as usize];
        let mut crossings: Vec<(f32, i32)> = Vec::new();
        for py in y0..y1 {
            coverage.fill(0.0);
            for s in 0..SUB {
                let sy = py as f32 + (s as f32 + 0.5) / SUB as f32;
                crossings.clear();
                for poly in polygons {
                    for i in 0..poly.len() {
                        let ((ax, ay), (bx, by)) = (map(&poly[i]), map(&poly[(i + 1) % poly.len()]));
                        if (ay <= sy) != (by <= sy) {
                            crossings.push((ax + (sy - ay) * (bx - ax) / (by - ay), if by > ay { 1 } else { -1 }));
                        }
                    }
                }
                crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
                let (mut winding, mut start) = (0, 0.0);
                for &(x, dir) in &crossings {
                    let before = winding;
                    winding += dir;
                    if before == 0 && winding != 0 {
                        start = x;
                    } else if before != 0 && winding == 0 {
                        let (a, b) = (start.max(x0 as f32), x.min(x1 as f32));
                        let mut px = a.floor() as i32;
                        while (px as f32) < b {
                            let overlap = b.min(px as f32 + 1.0) - a.max(px as f32);
                            coverage[(px - x0) as usize] += overlap.max(0.0);
                            px += 1;
                        }
                    }
                }
            }
            for (i, c) in coverage.iter().enumerate() {
                self.blend(x0 + i as i32, py, color, c / SUB as f32);
            }
        }
    }

    /// `y` is the top of the line.
    fn text(&mut self, style: &Text, x: i32, y: i32, s: &str) {
        let baseline = y + style.ascent();
        let mut pen = x as f32;
        let mut prev = None;
        for ch in s.chars() {
            if let Some(p) = prev {
                pen += style.font.horizontal_kern(p, ch, style.size).unwrap_or(0.0);
            }
            let (m, bitmap) = style.font.rasterize(ch, style.size);
            let gx = pen.round() as i32 + m.xmin;
            let gy = baseline - m.height as i32 - m.ymin;
            for row in 0..m.height {
                for col in 0..m.width {
                    let alpha = bitmap[row * m.width + col] as f32 / 255.0;
                    self.blend(gx + col as i32, gy + row as i32, style.color, alpha);
                }
            }
            pen += m.advance_width;
            prev = Some(ch);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn sample_cards() -> Vec<Card> {
        vec![
            Card {
                name: "lo".into(),
                description: None,
                kind: Kind::Loopback,
                up: true,
                mac: None,
                ipv4: Some((Ipv4Addr::LOCALHOST, 8)),
                default_route: false,
            },
            Card {
                name: "enp3s0".into(),
                description: None,
                kind: Kind::Wired,
                up: true,
                mac: Some([0xD0, 0x94, 0x66, 0xA1, 0xFF, 0xC7]),
                ipv4: Some((Ipv4Addr::new(192, 168, 100, 195), 24)),
                default_route: true,
            },
            Card {
                name: "wlp2s0".into(),
                description: None,
                kind: Kind::Wireless,
                up: true,
                mac: Some([0xD4, 0x6A, 0x6A, 0xF0, 0x61, 0xB5]),
                ipv4: Some((Ipv4Addr::new(192, 168, 100, 87), 24)),
                default_route: false,
            },
        ]
    }

    #[test]
    fn every_message_has_glyphs_in_the_embedded_font() {
        let fonts = Fonts::load();
        for lang in [Lang::En, Lang::PtBr, Lang::PtPt, Lang::Es] {
            let all = [
                Msg::Scan, Msg::Cards, Msg::Suggested, Msg::Wired, Msg::Wireless, Msg::Virtual,
                Msg::Loopback, Msg::Disconnected, Msg::NoIpv4, Msg::WirelessWarning,
                Msg::WirelessChosen, Msg::NoCard,
                Msg::CardNotFound("x"), Msg::CardNotUsable("x"), Msg::CardVirtual("x"),
                Msg::PressEnter, Msg::LabelName, Msg::LabelMac, Msg::LabelIp, Msg::LabelBroadcast,
                Msg::LabelPort, Msg::Usage,
            ];
            let mut chars: String = all.into_iter().map(|m| text(lang, m)).collect();
            chars += TITLE;
            chars += "·[]/";
            for font in [&fonts.regular, &fonts.bold] {
                for ch in chars.chars().filter(|c| !c.is_whitespace()) {
                    assert_ne!(font.lookup_glyph_index(ch), 0, "no glyph for {ch:?} ({lang:?})");
                }
            }
        }
    }

    #[test]
    fn clicking_a_row_picks_its_card() {
        let fonts = Fonts::load();
        let cards = sample_cards();
        let view = View { lang: Lang::En, cards: &cards, chosen: 1, hostname: Some("inspiron") };
        let frame = render(&fonts, &view, &fit(&fonts, &view, u32::MAX, 1.0));
        let (wired, wireless) = (frame.rows[0].0, frame.rows[1].0);
        assert_eq!(frame.rows.len(), 2, "loopback has no row");
        assert_eq!(frame.hit(wireless.x + 5, wireless.y + 5), Some(2));
        assert_eq!(frame.hit(wired.x + 5, wired.y + 5), Some(1));
        assert_eq!(frame.hit(0, 0), None);
    }

    #[test]
    fn the_window_fits_the_wireless_warning() {
        let fonts = Fonts::load();
        let cards = sample_cards();
        let wired = View { lang: Lang::PtPt, cards: &cards, chosen: 1, hostname: None };
        let wireless = View { chosen: 2, ..wired };
        let size = fit(&fonts, &wired, u32::MAX, 1.0);
        assert!(layout(&fonts, &wireless, &size, None) >= layout(&fonts, &wired, &size, None));
        assert_eq!(size.height, layout(&fonts, &wireless, &size, None) as u32);
    }

    #[test]
    fn a_short_screen_gets_a_smaller_qr() {
        let fonts = Fonts::load();
        let cards = sample_cards();
        let view = View { lang: Lang::En, cards: &cards, chosen: 1, hostname: Some("inspiron") };
        let roomy = fit(&fonts, &view, u32::MAX, 1.0);
        // Shrinking the QR only helps until the right column is the taller one.
        let short = fit(&fonts, &view, roomy.height - 20, 1.0);
        assert_eq!(roomy.module, MODULE_MAX as i32);
        assert!(short.module < MODULE_MAX as i32 && short.height <= roomy.height - 20);
        // A screen too short for anything still gets the smallest readable QR.
        assert_eq!(fit(&fonts, &view, 100, 1.0).module, MODULE_MIN);
    }

    #[test]
    fn scale_two_doubles_the_window() {
        let fonts = Fonts::load();
        let cards = sample_cards();
        let view = View { lang: Lang::En, cards: &cards, chosen: 1, hostname: Some("inspiron") };
        let one = fit(&fonts, &view, u32::MAX, 1.0);
        let two = fit(&fonts, &view, u32::MAX, 2.0);
        assert_eq!(two.module, 2 * one.module);
        // Text metrics round per line, so allow a few pixels either way.
        assert!((two.width as i32 - 2 * one.width as i32).abs() <= 4, "{one:?} {two:?}");
        assert!((two.height as i32 - 2 * one.height as i32).abs() <= 16, "{one:?} {two:?}");
        // Clicks land on the same rows, in physical pixels.
        let frame = render(&fonts, &view, &two);
        let wireless = frame.rows[1].0;
        assert_eq!(frame.hit(wireless.x + 10, wireless.y + 10), Some(2));
    }

    /// `cargo test -- --ignored png`: the window in each language, as target/ui-*.png.
    #[test]
    #[ignore]
    fn png() {
        let fonts = Fonts::load();
        let cards = sample_cards();
        for (lang, tag) in [(Lang::En, "en"), (Lang::PtBr, "pt-BR"), (Lang::PtPt, "pt-PT"), (Lang::Es, "es")] {
            for chosen in [1, 2] {
                let view = View { lang, cards: &cards, chosen, hostname: Some("inspiron") };
                // A 1366×768 laptop, less the panel and the title bar.
                let frame = render(&fonts, &view, &fit(&fonts, &view, 768 - 120, 1.0));
                let path = format!("target/ui-{tag}-{}.png", cards[chosen].name);
                std::fs::write(&path, encode_png(&frame.canvas)).unwrap();
                // The same at 150 %, as on most Windows laptops.
                let frame = render(&fonts, &view, &fit(&fonts, &view, 1080 - 120, 1.5));
                let path = format!("target/ui-{tag}-{}@1.5x.png", cards[chosen].name);
                std::fs::write(&path, encode_png(&frame.canvas)).unwrap();
            }
        }
    }

    /// Uncompressed PNG, enough to look at a frame.
    fn encode_png(c: &Canvas) -> Vec<u8> {
        fn crc(data: &[u8]) -> u32 {
            let mut crc = !0u32;
            for &b in data {
                crc ^= b as u32;
                for _ in 0..8 {
                    crc = if crc & 1 != 0 { 0xEDB88320 ^ (crc >> 1) } else { crc >> 1 };
                }
            }
            !crc
        }
        fn chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
            out.extend((data.len() as u32).to_be_bytes());
            let mut body = kind.to_vec();
            body.extend(data);
            out.extend(&body);
            out.extend(crc(&body).to_be_bytes());
        }
        let mut raw = Vec::new();
        for row in c.pixels.chunks(c.width as usize) {
            raw.push(0);
            for p in row {
                raw.extend([(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
            }
        }
        let mut zlib = vec![0x78, 0x01];
        let blocks: Vec<&[u8]> = raw.chunks(65535).collect();
        for (i, block) in blocks.iter().enumerate() {
            zlib.push((i + 1 == blocks.len()) as u8);
            let len = block.len() as u16;
            zlib.extend(len.to_le_bytes());
            zlib.extend((!len).to_le_bytes());
            zlib.extend(*block);
        }
        let (mut a, mut b) = (1u32, 0u32);
        for &byte in &raw {
            a = (a + byte as u32) % 65521;
            b = (b + a) % 65521;
        }
        zlib.extend(((b << 16) | a).to_be_bytes());
        let mut ihdr = Vec::new();
        ihdr.extend(c.width.to_be_bytes());
        ihdr.extend(c.height.to_be_bytes());
        ihdr.extend([8, 2, 0, 0, 0]);
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        chunk(&mut out, b"IHDR", &ihdr);
        chunk(&mut out, b"IDAT", &zlib);
        chunk(&mut out, b"IEND", &[]);
        out
    }
}
