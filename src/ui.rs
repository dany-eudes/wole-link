//! The window's contents and what clicks do, drawn into a pixel buffer. The same code serves
//! X11 and Win32: each only shows the buffer, passes clicks and Esc to `App` and carries out the
//! `Effect` it returns.

use crate::i18n::{text, Lang, Msg};
use crate::icons::{self, Icon};
use crate::link;
use crate::net::{Card, Kind};
use crate::wordmark;
use fontdue::{Font, FontSettings};
use qrcodegen::{QrCode, QrCodeEcc};

const REGULAR: &[u8] = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../assets/fonts/NotoSans-Bold.ttf");

pub const TITLE: &str = "WOL•E Link";
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
// Under the pointer: a menu row or ☰, a card that isn't the chosen one, the About button.
const HOVER: u32 = 0xE8EEF4;
const CARD_HOVER: u32 = 0xF0F7FA;
const CARD_HOVER_BORDER: u32 = 0x8CC3D3;
const ACCENT_HOVER: u32 = 0x0B6E8B;
/// Corner radii, as in Windows 11: panels and cards, then buttons and menu rows.
const RADIUS: f32 = 8.0;
const RADIUS_SMALL: f32 = 4.0;

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

/// What a click on a spot does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Card(usize),
    Menu,
    SetLang(Lang),
    About,
    Open(String),
    /// Add, update or remove the Linux applications-menu entry.
    DesktopEntry,
    CloseOverlay,
    /// Swallows the click (the About panel itself).
    Nothing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    None,
    Menu,
    About,
}

pub struct Frame {
    pub canvas: Canvas,
    /// Click targets, first match wins.
    targets: Vec<(Rect, Action)>,
}

impl Frame {
    pub fn hit(&self, x: i32, y: i32) -> Option<Action> {
        self.targets.iter().find(|(r, _)| r.contains(x, y)).map(|(_, a)| a.clone())
    }

    /// The area a click at (`x`, `y`) would land on.
    fn target_at(&self, x: i32, y: i32) -> Option<Rect> {
        self.targets.iter().find(|(r, _)| r.contains(x, y)).map(|(r, _)| *r)
    }
}

#[derive(Clone, Copy)]
pub struct View<'a> {
    pub lang: Lang,
    pub cards: &'a [Card],
    /// Index into `cards`; must be a usable card.
    pub chosen: usize,
    pub hostname: Option<&'a str>,
    pub overlay: Overlay,
    /// The Linux applications-menu entry; `None` where there is no such thing (Windows).
    pub desktop: Option<crate::desktop::State>,
    /// The click target under the pointer, drawn highlighted.
    pub hover: Option<Rect>,
}

/// What the window must do after a click or Esc.
#[derive(Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    Redraw,
    Open(String),
    Close,
}

/// Everything the window shows and how it changes; the platform code only draws `frame()`.
pub struct App {
    fonts: Fonts,
    lang: Lang,
    cards: Vec<Card>,
    chosen: usize,
    hostname: Option<String>,
    overlay: Overlay,
    desktop: Option<crate::desktop::State>,
    /// Where the pointer is over the window, and the click target it is on.
    pointer: Option<(i32, i32)>,
    hover: Option<Rect>,
    size: Size,
    frame: Frame,
}

impl App {
    /// `max_height` and `scale` in physical pixels, from the screen the window opens on.
    pub fn new(start: Start, max_height: u32, scale: f32) -> App {
        let fonts = Fonts::load();
        let desktop = crate::desktop::available().then(crate::desktop::state);
        let view = View {
            lang: start.lang,
            cards: &start.cards,
            chosen: start.chosen,
            hostname: start.hostname.as_deref(),
            overlay: Overlay::None,
            desktop,
            hover: None,
        };
        let size = fit(&fonts, &view, max_height, scale);
        let frame = render(&fonts, &view, &size);
        App {
            fonts,
            lang: start.lang,
            cards: start.cards,
            chosen: start.chosen,
            hostname: start.hostname,
            overlay: Overlay::None,
            desktop,
            pointer: None,
            hover: None,
            size,
            frame,
        }
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    fn view(&self) -> View<'_> {
        View {
            lang: self.lang,
            cards: &self.cards,
            chosen: self.chosen,
            hostname: self.hostname.as_deref(),
            overlay: self.overlay,
            desktop: self.desktop,
            hover: self.hover,
        }
    }

    /// A click can move what lies under the pointer (the menu opens over the cards): the
    /// highlight follows in the same redraw.
    fn redraw(&mut self) -> Effect {
        self.frame = render(&self.fonts, &self.view(), &self.size);
        let hover = self.hovered();
        if hover != self.hover {
            self.hover = hover;
            self.frame = render(&self.fonts, &self.view(), &self.size);
        }
        Effect::Redraw
    }

    fn hovered(&self) -> Option<Rect> {
        self.pointer.and_then(|(x, y)| self.frame.target_at(x, y))
    }

    /// The pointer moved to (`x`, `y`), or left the window with `None`. Redraws only when it
    /// crosses into another click target.
    pub fn pointer(&mut self, at: Option<(i32, i32)>) -> Effect {
        self.pointer = at;
        let hover = self.hovered();
        if hover == self.hover {
            return Effect::None;
        }
        self.hover = hover;
        self.frame = render(&self.fonts, &self.view(), &self.size);
        Effect::Redraw
    }

    pub fn click(&mut self, x: i32, y: i32) -> Effect {
        // A touch screen clicks without moving the pointer first.
        self.pointer = Some((x, y));
        let Some(action) = self.frame.hit(x, y) else { return Effect::None };
        match action {
            Action::Card(i) if i == self.chosen => Effect::None,
            Action::Card(i) => {
                self.chosen = i;
                self.redraw()
            }
            Action::Menu => {
                self.overlay = if self.overlay == Overlay::Menu { Overlay::None } else { Overlay::Menu };
                self.redraw()
            }
            Action::SetLang(lang) => {
                self.lang = lang;
                crate::settings::save_lang(lang);
                self.overlay = Overlay::None;
                self.redraw()
            }
            Action::About => {
                self.overlay = Overlay::About;
                self.redraw()
            }
            Action::Open(url) => {
                self.overlay = Overlay::None;
                self.redraw();
                Effect::Open(url)
            }
            Action::DesktopEntry => {
                let _ = match self.desktop {
                    Some(crate::desktop::State::Installed) => crate::desktop::remove(),
                    _ => crate::desktop::install(),
                };
                self.desktop = Some(crate::desktop::state());
                self.overlay = Overlay::None;
                self.redraw()
            }
            Action::CloseOverlay => {
                self.overlay = Overlay::None;
                self.redraw()
            }
            Action::Nothing => Effect::None,
        }
    }

    /// Esc closes the menu or About first, the window only when nothing is open.
    pub fn escape(&mut self) -> Effect {
        if self.overlay == Overlay::None {
            return Effect::Close;
        }
        self.overlay = Overlay::None;
        self.redraw()
    }
}

/// What `main` hands to the window.
pub struct Start {
    pub lang: Lang,
    pub cards: Vec<Card>,
    pub chosen: usize,
    pub hostname: Option<String>,
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
/// physical pixels. The height is the tallest of every view the user can reach (each card, each
/// language, the menu and About open), so the window never changes size.
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
        size.height = 0;
        for &chosen in &usable {
            for lang in Lang::ALL {
                for overlay in [Overlay::None, Overlay::Menu, Overlay::About] {
                    let v = View { chosen, lang, overlay, ..*view };
                    size.height = size.height.max(layout(fonts, &v, &size, None) as u32);
                }
            }
        }
        if size.height <= max_height {
            break;
        }
    }
    size
}

pub fn render(fonts: &Fonts, view: &View, size: &Size) -> Frame {
    let mut canvas = Canvas::new(size.width, size.height, BACKGROUND);
    let mut targets = Vec::new();
    layout(fonts, view, size, Some((&mut canvas, &mut targets)));
    Frame { canvas, targets }
}

type Target<'a> = Option<(&'a mut Canvas, &'a mut Vec<(Rect, Action)>)>;

/// Title and ☰ on top; QR on the left, data and cards on the right; the link at the bottom; the
/// menu or About over it all. Returns the height, and draws too when given a canvas.
fn layout(fonts: &Fonts, view: &View, size: &Size, mut target: Target) -> i32 {
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

    // The WOL•E wordmark and "Link"; the ☰ button on the right, on the same line.
    let mark_height = 34.0 * sc;
    draw!(|c| c.title(fonts, pad, y, mark_height, sc));
    let button = px(36);
    let menu_button = Rect {
        x: size.width as i32 - pad - button,
        y: y + (mark_height as i32 - button) / 2,
        w: button,
        h: button,
    };
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
            c.round_rect(pad, top, side, side, RADIUS * sc, SURFACE, 1.0);
            c.round_outline(pad, top, side, side, RADIUS * sc, BORDER);
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
            c.round_rect(x, y, right, h, RADIUS * sc, WARNING_BACKGROUND, 1.0);
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
            let hovered = usable && !chosen && view.hover == Some(rect);
            let (fill, border) = match (chosen, hovered) {
                (true, _) => (SELECTED, ACCENT),
                (false, true) => (CARD_HOVER, CARD_HOVER_BORDER),
                (false, false) => (SURFACE, BORDER),
            };
            c.round_rect(rect.x, rect.y, rect.w, rect.h, RADIUS * sc, fill, 1.0);
            c.round_outline(rect.x, rect.y, rect.w, rect.h, RADIUS * sc, border);
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
                rows.push((rect, Action::Card(i)));
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
    let mut height = y + pad;

    // ☰: three bars, highlighted while the menu is open or under the pointer.
    if let Some((c, targets)) = target.as_mut() {
        let b = menu_button;
        let fill = if view.overlay == Overlay::Menu {
            Some(SELECTED)
        } else {
            (view.hover == Some(b)).then_some(HOVER)
        };
        if let Some(fill) = fill {
            c.round_rect(b.x, b.y, b.w, b.h, RADIUS_SMALL * sc, fill, 1.0);
        }
        let (bar_w, bar_h) = (px(18), px(2).max(2));
        for i in -1..=1 {
            let by = b.y + b.h / 2 - bar_h / 2 + i * px(6);
            c.rect(b.x + (b.w - bar_w) / 2, by, bar_w, bar_h, TEXT);
        }
        targets.push((b, Action::Menu));
    }

    // The menu or About sits on top: its targets replace the window's, and a click anywhere
    // else just closes it.
    match view.overlay {
        Overlay::None => {}
        Overlay::Menu => height = height.max(menu(fonts, view, size, menu_button, &mut target) + pad),
        Overlay::About => height = height.max(about(fonts, view, size, &mut target) + 2 * pad),
    }
    height
}

/// Site and Play Store pages in the window's language.
fn site_url(lang: Lang) -> String {
    let path = match lang {
        Lang::En => "",
        Lang::PtBr => "pt-br/",
        Lang::PtPt => "pt-pt/",
        Lang::Es => "es/",
    };
    format!("https://wole.celerasoft.com/{path}")
}

fn play_url(lang: Lang) -> String {
    let hl = if lang == Lang::Es { "es-419" } else { lang.tag() };
    format!("https://play.google.com/store/apps/details?id=com.celerasoft.wole&hl={hl}")
}

const COFFEE_URL: &str = "https://buymeacoffee.com/celeraman";
const SUPPORT_MAIL: &str = "mailto:celeraman@gmail.com?subject=WOL%E2%80%A2E%20Link";
pub const COPYRIGHT: &str = "© 2026 @celeraman";

enum Item {
    Heading(String, Icon),
    Choice(String, bool, Action),
    Entry(String, Icon, Action),
    Separator,
}

/// The drop-down under ☰, right-aligned with it: the languages; About, the one thing that opens
/// in here; the links out; on Linux, the applications-menu entry. Returns its bottom.
fn menu(fonts: &Fonts, view: &View, size: &Size, button: Rect, target: &mut Target) -> i32 {
    let lang = view.lang;
    let sc = size.scale;
    let px = |v: i32| (v as f32 * sc).round() as i32;
    let mut items = vec![Item::Heading(text(lang, Msg::MenuLanguage), Icon::Globe)];
    for l in Lang::ALL {
        items.push(Item::Choice(l.native_name().into(), l == lang, Action::SetLang(l)));
    }
    items.push(Item::Separator);
    items.push(Item::Entry(text(lang, Msg::MenuAbout), Icon::Info, Action::About));
    items.push(Item::Separator);
    items.push(Item::Entry(text(lang, Msg::MenuSite), Icon::External, Action::Open(site_url(lang))));
    items.push(Item::Entry(text(lang, Msg::MenuPlay), Icon::Play, Action::Open(play_url(lang))));
    items.push(Item::Entry(text(lang, Msg::MenuCoffee), Icon::Coffee, Action::Open(COFFEE_URL.into())));
    items.push(Item::Entry(text(lang, Msg::MenuContact), Icon::Mail, Action::Open(SUPPORT_MAIL.into())));
    if let Some(state) = view.desktop {
        let msg = match state {
            crate::desktop::State::Missing => Msg::MenuAddEntry,
            crate::desktop::State::Installed => Msg::MenuRemoveEntry,
            crate::desktop::State::Moved => Msg::MenuUpdateEntry,
        };
        items.push(Item::Separator);
        items.push(Item::Entry(text(lang, msg), Icon::Apps, Action::DesktopEntry));
    }

    let entry = Text::new(&fonts.regular, 14.0 * sc, TEXT);
    let heading = Text::new(&fonts.bold, 12.0 * sc, TEXT_SECONDARY);
    // Icons and the ✓ share a column; every text starts at `indent`.
    let (row, indent, icon_x) = (px(32), px(44), px(24));
    let width = items
        .iter()
        .map(|item| match item {
            Item::Choice(t, ..) | Item::Entry(t, ..) => indent + entry.width(t) + px(20),
            Item::Heading(t, _) => indent + heading.width(t) + px(20),
            Item::Separator => 0,
        })
        .max()
        .unwrap_or(0)
        .max(px(240));
    let height: i32 = items
        .iter()
        .map(|item| if matches!(item, Item::Separator) { px(9) } else { row })
        .sum::<i32>()
        + px(12);
    let panel = Rect { x: button.x + button.w - width, y: button.y + button.h + px(4), w: width, h: height };

    if let Some((c, targets)) = target.as_mut() {
        targets.clear();
        // A soft shadow, then the panel.
        for i in 1..=px(4) {
            c.round_rect(panel.x + i, panel.y + i, panel.w, panel.h, RADIUS * sc, 0x000000, 0.05);
        }
        c.round_rect(panel.x, panel.y, panel.w, panel.h, RADIUS * sc, SURFACE, 1.0);
        c.round_outline(panel.x, panel.y, panel.w, panel.h, RADIUS * sc, BORDER);
        let mut y = panel.y + px(6);
        for item in &items {
            let rect = Rect { x: panel.x, y, w: panel.w, h: row };
            let (cx, cy) = (panel.x + icon_x, y + row / 2);
            match item {
                Item::Separator => {
                    c.rect(panel.x + px(12), y + px(4), panel.w - px(24), 1, BORDER);
                    y += px(9);
                    continue;
                }
                Item::Heading(t, icon) => {
                    c.icon(*icon, cx, cy, sc, TEXT_SECONDARY);
                    c.text(&heading, panel.x + indent, y + (row - heading.line_height()) / 2, t);
                }
                Item::Choice(t, on, action) => {
                    c.hover_row(rect, view.hover, sc);
                    if *on {
                        // Noto Sans has no ✓: a drawn tick, in the accent color.
                        let (cx, cy, u) = (cx as f32, cy as f32, 4.0 * sc);
                        let tick: &[(f32, f32)] = &[
                            (cx - 1.6 * u, cy - 0.1 * u),
                            (cx - 0.5 * u, cy + 1.0 * u),
                            (cx + 1.7 * u, cy - 1.3 * u),
                            (cx + 1.7 * u - 0.55 * u, cy - 1.3 * u - 0.45 * u),
                            (cx - 0.5 * u, cy + 1.0 * u - 0.75 * u),
                            (cx - 1.6 * u + 0.5 * u, cy - 0.1 * u - 0.55 * u),
                        ];
                        c.fill(&[tick], 0.0, 0.0, 1.0, ACCENT);
                    }
                    c.text(&entry, panel.x + indent, y + (row - entry.line_height()) / 2, t);
                    targets.push((rect, action.clone()));
                }
                Item::Entry(t, icon, action) => {
                    c.hover_row(rect, view.hover, sc);
                    c.icon(*icon, cx, cy, sc, TEXT_SECONDARY);
                    c.text(&entry, panel.x + indent, y + (row - entry.line_height()) / 2, t);
                    targets.push((rect, action.clone()));
                }
            }
            y += row;
        }
        targets.push((button, Action::Menu));
        targets.push((Rect { x: 0, y: 0, w: size.width as i32, h: size.height as i32 }, Action::CloseOverlay));
    }
    panel.y + panel.h
}

/// About, over a dimmed window. Returns its height.
fn about(fonts: &Fonts, view: &View, size: &Size, target: &mut Target) -> i32 {
    let lang = view.lang;
    let sc = size.scale;
    let px = |v: i32| (v as f32 * sc).round() as i32;
    let width = px(440).min(size.width as i32 - 2 * px(PAD));
    let inner = width - 2 * px(28);
    let body = Text::new(&fonts.regular, 14.0 * sc, TEXT);
    let small = Text::new(&fonts.regular, 13.0 * sc, TEXT_SECONDARY);
    let button_text = Text::new(&fonts.bold, 14.0 * sc, SURFACE);
    let version = text(lang, Msg::AboutVersion(env!("CARGO_PKG_VERSION")));
    let description = body.wrap(&text(lang, Msg::AboutText), inner);
    let lines: Vec<(String, &Text)> = [
        (COPYRIGHT.to_string(), &small),
        (text(lang, Msg::AboutLicense), &small),
        (text(lang, Msg::AboutFont), &small),
    ]
    .into_iter()
    .flat_map(|(t, style)| style.wrap(&t, inner).into_iter().map(move |l| (l, style)))
    .collect();
    let mark_height = 28.0 * sc;
    let close = text(lang, Msg::Close);
    let (button_w, button_h) = (button_text.width(&close) + px(40), px(36));
    let height = px(28)
        + mark_height as i32
        + px(8)
        + small.line_height()
        + px(14)
        + description.len() as i32 * body.line_height()
        + px(14)
        + lines.iter().map(|(_, s)| s.line_height()).sum::<i32>()
        + px(22)
        + button_h
        + px(24);

    if let Some((c, targets)) = target.as_mut() {
        let x = (size.width as i32 - width) / 2;
        let top = ((size.height as i32 - height) / 2).max(px(PAD));
        c.shade(0, 0, size.width as i32, size.height as i32, 0.35);
        c.round_rect(x, top, width, height, RADIUS * sc, SURFACE, 1.0);
        c.round_outline(x, top, width, height, RADIUS * sc, BORDER);
        let (lx, mut y) = (x + px(28), top + px(28));
        c.title(fonts, lx, y, mark_height, sc);
        y += mark_height as i32 + px(8);
        c.text(&small, lx, y, &version);
        y += small.line_height() + px(14);
        for line in &description {
            c.text(&body, lx, y, line);
            y += body.line_height();
        }
        y += px(14);
        for (line, style) in &lines {
            c.text(style, lx, y, line);
            y += style.line_height();
        }
        y += px(22);
        let b = Rect { x: x + width - px(28) - button_w, y, w: button_w, h: button_h };
        let fill = if view.hover == Some(b) { ACCENT_HOVER } else { ACCENT };
        c.round_rect(b.x, b.y, b.w, b.h, RADIUS_SMALL * sc, fill, 1.0);
        c.text(&button_text, b.x + px(20), b.y + (b.h - button_text.line_height()) / 2, &close);
        targets.clear();
        targets.push((b, Action::CloseOverlay));
        let panel = Rect { x, y: top, w: width, h: height };
        // Clicks on the panel do nothing; outside it, they close it.
        targets.push((panel, Action::Nothing));
        targets.push((Rect { x: 0, y: 0, w: size.width as i32, h: size.height as i32 }, Action::CloseOverlay));
    }
    height
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

    /// A rectangle with corners of radius `r`, anti-aliased; an `alpha` under 1 blends it in
    /// (the menu's shadow).
    #[allow(clippy::too_many_arguments)]
    fn round_rect(&mut self, x: i32, y: i32, w: i32, h: i32, r: f32, color: u32, alpha: f32) {
        self.round_shape(x, y, w, h, r, color, |d| (0.5 - d).clamp(0.0, 1.0) * alpha);
    }

    /// The 1 px border just inside `round_rect`'s edge.
    fn round_outline(&mut self, x: i32, y: i32, w: i32, h: i32, r: f32, color: u32) {
        self.round_shape(x, y, w, h, r, color, |d| (1.0 - (d + 0.5).abs()).clamp(0.0, 1.0));
    }

    /// Paints each pixel of the box by its coverage, from its centre's signed distance to the
    /// rounded edge (negative inside).
    #[allow(clippy::too_many_arguments)]
    fn round_shape(&mut self, x: i32, y: i32, w: i32, h: i32, r: f32, color: u32, coverage: impl Fn(f32) -> f32) {
        let (hx, hy) = (w as f32 / 2.0, h as f32 / 2.0);
        let (cx, cy) = (x as f32 + hx, y as f32 + hy);
        let r = r.min(hx).min(hy);
        for py in y.max(0)..(y + h).min(self.height as i32) {
            for px in x.max(0)..(x + w).min(self.width as i32) {
                let qx = (px as f32 + 0.5 - cx).abs() - (hx - r);
                let qy = (py as f32 + 0.5 - cy).abs() - (hy - r);
                let d = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r;
                self.blend(px, py, color, coverage(d));
            }
        }
    }

    /// A menu row's highlight when the pointer is on it, inset from the panel's sides.
    fn hover_row(&mut self, row: Rect, hover: Option<Rect>, sc: f32) {
        if hover == Some(row) {
            let (inset, gap) = ((4.0 * sc).round() as i32, (1.0 * sc).round() as i32);
            self.round_rect(row.x + inset, row.y + gap, row.w - 2 * inset, row.h - 2 * gap, RADIUS_SMALL * sc, HOVER, 1.0);
        }
    }

    /// A 16 × 16 icon centred on (`cx`, `cy`).
    fn icon(&mut self, icon: Icon, cx: i32, cy: i32, sc: f32, color: u32) {
        let shapes = icons::polygons(icon);
        let shapes: Vec<&[(f32, f32)]> = shapes.iter().map(Vec::as_slice).collect();
        self.fill(&shapes, cx as f32, cy as f32, sc, color);
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

    /// The WOL•E wordmark at `mark_height`, then "Link" centred on its height.
    fn title(&mut self, fonts: &Fonts, x: i32, y: i32, mark_height: f32, sc: f32) {
        let k = mark_height / wordmark::HEIGHT;
        for (color, polygons) in wordmark::SHAPES {
            self.fill(polygons, x as f32, y as f32, k, *color);
        }
        let size = mark_height * 25.0 / 34.0;
        let title = Text::new(&fonts.bold, size, TEXT);
        // The capitals' middle on the mark's middle.
        let (top, bottom) = wordmark::SHAPES
            .iter()
            .flat_map(|(_, polygons)| polygons.iter().flat_map(|p| p.iter()))
            .fold((f32::MAX, f32::MIN), |(t, b), &(_, py)| (t.min(py), b.max(py)));
        let middle = y as f32 + (top + bottom) / 2.0 * k;
        let baseline = middle + fonts.bold.metrics('L', size).height as f32 / 2.0;
        let after = x + (wordmark::WIDTH * k).ceil() as i32 + (10.0 * sc).round() as i32;
        self.text(&title, after, baseline.round() as i32 - title.ascent(), "Link");
    }

    /// Darkens a rectangle by `amount` (0 to 1).
    fn shade(&mut self, x: i32, y: i32, w: i32, h: i32, amount: f32) {
        for py in y.max(0)..(y + h).min(self.height as i32) {
            for px in x.max(0)..(x + w).min(self.width as i32) {
                self.blend(px, py, 0x000000, amount);
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
                Msg::LabelPort, Msg::MenuLanguage, Msg::MenuAbout, Msg::MenuSite, Msg::MenuPlay,
                Msg::MenuCoffee, Msg::MenuContact, Msg::MenuAddEntry, Msg::MenuRemoveEntry,
                Msg::MenuUpdateEntry, Msg::AboutVersion("1.0"), Msg::AboutText, Msg::AboutLicense,
                Msg::AboutFont, Msg::Close, Msg::Usage("wole-link"), Msg::UseCli,
            ];
            let mut chars: String = all.into_iter().map(|m| text(lang, m)).collect();
            chars += TITLE;
            chars += COPYRIGHT;
            chars += "·[]/";
            chars.extend(Lang::ALL.map(Lang::native_name).concat().chars());
            for font in [&fonts.regular, &fonts.bold] {
                for ch in chars.chars().filter(|c| !c.is_whitespace()) {
                    assert_ne!(font.lookup_glyph_index(ch), 0, "no glyph for {ch:?} ({lang:?})");
                }
            }
        }
    }

    fn view(cards: &[Card], lang: Lang, chosen: usize) -> View<'_> {
        View {
            lang,
            cards,
            chosen,
            hostname: Some("inspiron"),
            overlay: Overlay::None,
            desktop: Some(crate::desktop::State::Missing),
            hover: None,
        }
    }

    fn target(frame: &Frame, action: &Action) -> Rect {
        frame.targets.iter().find(|(_, a)| a == action).map(|(r, _)| *r).expect("target")
    }

    fn app(lang: Lang) -> App {
        let start = Start { lang, cards: sample_cards(), chosen: 1, hostname: Some("inspiron".into()) };
        App::new(start, u32::MAX, 1.0)
    }

    fn click_on(app: &mut App, action: &Action) -> Effect {
        let r = target(app.frame(), action);
        app.click(r.x + r.w / 2, r.y + r.h / 2)
    }

    #[test]
    fn clicking_a_row_picks_its_card() {
        let fonts = Fonts::load();
        let cards = sample_cards();
        let v = view(&cards, Lang::En, 1);
        let frame = render(&fonts, &v, &fit(&fonts, &v, u32::MAX, 1.0));
        let cards_targets = frame.targets.iter().filter(|(_, a)| matches!(a, Action::Card(_))).count();
        assert_eq!(cards_targets, 2, "loopback has no row");
        let wireless = target(&frame, &Action::Card(2));
        assert_eq!(frame.hit(wireless.x + 5, wireless.y + 5), Some(Action::Card(2)));
        assert_eq!(frame.hit(0, 0), None);
    }

    #[test]
    fn the_window_fits_every_view() {
        let fonts = Fonts::load();
        let cards = sample_cards();
        let v = view(&cards, Lang::PtPt, 1);
        let size = fit(&fonts, &v, u32::MAX, 1.0);
        for lang in Lang::ALL {
            for chosen in [1, 2] {
                for overlay in [Overlay::None, Overlay::Menu, Overlay::About] {
                    let other = View { lang, chosen, overlay, ..v };
                    assert!(layout(&fonts, &other, &size, None) as u32 <= size.height, "{lang:?} {overlay:?}");
                }
            }
        }
    }

    #[test]
    fn a_short_screen_gets_a_smaller_qr() {
        let fonts = Fonts::load();
        let cards = sample_cards();
        let v = view(&cards, Lang::En, 1);
        let roomy = fit(&fonts, &v, u32::MAX, 1.0);
        assert_eq!(roomy.module, MODULE_MAX as i32);
        // A screen too short for anything still gets the smallest readable QR.
        let tiny = fit(&fonts, &v, 100, 1.0);
        assert_eq!(tiny.module, MODULE_MIN);
        // Narrower, though not always shorter: the text above and the link below wrap more.
        assert!(tiny.width < roomy.width);
    }

    #[test]
    fn scale_two_doubles_the_window() {
        let fonts = Fonts::load();
        let cards = sample_cards();
        let v = view(&cards, Lang::En, 1);
        let one = fit(&fonts, &v, u32::MAX, 1.0);
        let two = fit(&fonts, &v, u32::MAX, 2.0);
        assert_eq!(two.module, 2 * one.module);
        // Text metrics round per line, so allow a few pixels either way.
        assert!((two.width as i32 - 2 * one.width as i32).abs() <= 4, "{one:?} {two:?}");
        assert!((two.height as i32 - 2 * one.height as i32).abs() <= 24, "{one:?} {two:?}");
        // Clicks land on the same rows, in physical pixels.
        let frame = render(&fonts, &v, &two);
        let wireless = target(&frame, &Action::Card(2));
        assert_eq!(frame.hit(wireless.x + 10, wireless.y + 10), Some(Action::Card(2)));
    }

    #[test]
    fn the_menu_opens_and_picks_a_language() {
        let mut app = app(Lang::En);
        assert_eq!(click_on(&mut app, &Action::Menu), Effect::Redraw);
        assert_eq!(app.overlay, Overlay::Menu);
        assert_eq!(click_on(&mut app, &Action::SetLang(Lang::Es)), Effect::Redraw);
        assert_eq!((app.lang, app.overlay), (Lang::Es, Overlay::None));
    }

    #[test]
    fn a_click_outside_the_menu_only_closes_it() {
        let mut app = app(Lang::En);
        let wireless = target(app.frame(), &Action::Card(2));
        click_on(&mut app, &Action::Menu);
        // The wireless row sits under the open menu's reach? Click a spot no menu item covers.
        let panel_left = app.frame().targets.iter().filter_map(|(r, a)| matches!(a, Action::SetLang(_)).then_some(r.x)).min().unwrap();
        assert!(wireless.x < panel_left);
        app.click(wireless.x + 2, wireless.y + 2);
        assert_eq!((app.overlay, app.chosen), (Overlay::None, 1), "closed, card unchanged");
    }

    #[test]
    fn escape_closes_about_then_the_window() {
        let mut app = app(Lang::PtBr);
        click_on(&mut app, &Action::Menu);
        click_on(&mut app, &Action::About);
        assert_eq!(app.overlay, Overlay::About);
        assert_eq!(app.escape(), Effect::Redraw);
        assert_eq!(app.overlay, Overlay::None);
        assert_eq!(app.escape(), Effect::Close);
    }

    #[test]
    fn links_close_the_menu_and_open() {
        let mut app = app(Lang::PtPt);
        click_on(&mut app, &Action::Menu);
        let site = Action::Open("https://wole.celerasoft.com/pt-pt/".into());
        assert_eq!(click_on(&mut app, &site), Effect::Open("https://wole.celerasoft.com/pt-pt/".into()));
        assert_eq!(app.overlay, Overlay::None);
    }

    #[test]
    fn the_pointer_highlights_a_card_once() {
        let mut app = app(Lang::En);
        let wireless = target(app.frame(), &Action::Card(2));
        let inside = (wireless.x + wireless.w / 2, wireless.y + 3);
        let pixel = |app: &App| app.frame().canvas.pixels[(inside.1 as u32 * app.size().width + inside.0 as u32) as usize];
        assert_eq!(pixel(&app), SURFACE);
        assert_eq!(app.pointer(Some(inside)), Effect::Redraw);
        assert_eq!(pixel(&app), CARD_HOVER);
        // Moving within the same card draws nothing new.
        assert_eq!(app.pointer(Some((inside.0 + 5, inside.1 + 5))), Effect::None);
        assert_eq!(app.pointer(None), Effect::Redraw);
        assert_eq!(pixel(&app), SURFACE);
        // Nothing to click there.
        assert_eq!(app.pointer(Some((0, 0))), Effect::None);
    }

    #[test]
    fn the_chosen_card_is_not_highlighted() {
        let mut app = app(Lang::En);
        let wired = target(app.frame(), &Action::Card(1));
        app.pointer(Some((wired.x + wired.w / 2, wired.y + 3)));
        let i = ((wired.y + 3) as u32 * app.size().width + (wired.x + wired.w / 2) as u32) as usize;
        assert_eq!(app.frame().canvas.pixels[i], SELECTED);
    }

    #[test]
    fn the_highlight_follows_a_click_that_opens_the_menu() {
        let mut app = app(Lang::En);
        click_on(&mut app, &Action::Menu);
        // The pointer is still on ☰, now a target of the open menu too.
        assert_eq!(app.hover, Some(target(app.frame(), &Action::Menu)));
        let about = target(app.frame(), &Action::About);
        assert_eq!(app.pointer(Some((about.x + 10, about.y + about.h / 2))), Effect::Redraw);
        assert_eq!(app.hover, Some(about));
        // About opens over the spot: nothing clickable there now but the backdrop.
        click_on(&mut app, &Action::About);
        assert_ne!(app.hover, Some(about));
    }

    /// `cargo test -- --ignored png`: the window in each language, as target/ui-*.png, with the
    /// menu and About open too.
    #[test]
    #[ignore]
    fn png() {
        let fonts = Fonts::load();
        let cards = sample_cards();
        for lang in Lang::ALL {
            for chosen in [1, 2] {
                for (overlay, suffix) in [(Overlay::None, ""), (Overlay::Menu, "-menu"), (Overlay::About, "-about")] {
                    if chosen == 2 && overlay != Overlay::None {
                        continue;
                    }
                    let v = View { overlay, ..view(&cards, lang, chosen) };
                    for (scale, max, tag) in [(1.0, 768 - 120, ""), (1.5, 1080 - 120, "@1.5x")] {
                        let size = fit(&fonts, &v, max, scale);
                        // The pointer on the other card, or on About in the menu.
                        let under = match overlay {
                            Overlay::None => Some(Action::Card(2)),
                            Overlay::Menu => Some(Action::About),
                            Overlay::About => None,
                        };
                        let hover = under.map(|a| target(&render(&fonts, &v, &size), &a));
                        let frame = render(&fonts, &View { hover, ..v }, &size);
                        let path = format!("target/ui-{}-{}{suffix}{tag}.png", lang.tag(), cards[chosen].name);
                        std::fs::write(&path, encode_png(&frame.canvas)).unwrap();
                    }
                }
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
