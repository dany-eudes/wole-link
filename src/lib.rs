//! Everything behind the two programs: `wole-link` (the window; on Windows a window program that
//! borrows the terminal it was started from for `--text` and `--list`) and `wole-link-cli` (a
//! console program, terminal only: on Windows the prompt waits for it and `>` works in PowerShell).

mod desktop;
mod gui;
mod i18n;
mod link;
mod net;
mod open;
mod qr;
mod settings;
mod ui;
mod wordmark;

use i18n::{text, Lang, Msg};
use net::{Card, Kind};
use std::io::IsTerminal;
use std::process::ExitCode;

struct Args {
    card: Option<String>,
    list_only: bool,
    text: bool,
    lang: Option<Lang>,
    help: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args { card: None, list_only: false, text: false, lang: None, help: false };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--card" => args.card = Some(it.next().ok_or("--card")?),
            "--list" => args.list_only = true,
            "--text" => args.text = true,
            "--lang" => args.lang = Some(i18n::parse(&it.next().ok_or("--lang")?)),
            "-h" | "--help" => args.help = true,
            other => return Err(other.to_string()),
        }
    }
    Ok(args)
}

/// `cli`: the terminal program, which never opens the window.
pub fn main(cli: bool) -> ExitCode {
    let code = run(cli);
    if console::owns_window() {
        println!("\n{}", text(chosen_lang(parse_args().ok().and_then(|a| a.lang)), Msg::PressEnter));
        let _ = std::io::stdin().read_line(&mut String::new());
    }
    console::restore();
    code
}

/// `--lang`, else the one picked in the window's menu last time, else the system's.
fn chosen_lang(flag: Option<Lang>) -> Lang {
    flag.or_else(settings::saved_lang).unwrap_or_else(i18n::detect)
}

fn run(cli: bool) -> ExitCode {
    let args = parse_args();
    let lang = chosen_lang(args.as_ref().ok().and_then(|a| a.lang));
    let cards = net::list();
    let hostname = net::hostname();

    // The window, unless the terminal was asked for. A `--card` that can't be used goes to the
    // terminal, which says why.
    if let Ok(args) = &args {
        if !cli && !args.text && !args.list_only && !args.help {
            let chosen = match &args.card {
                Some(name) => cards.iter().position(|c| &c.name == name).filter(|&i| cards[i].usable()),
                None => net::suggest(&cards),
            };
            match chosen {
                Some(chosen) => {
                    let start = ui::Start { lang, cards: cards.clone(), chosen, hostname: hostname.clone() };
                    match gui::run(start) {
                        Ok(()) => return ExitCode::SUCCESS,
                        // No display, or none we can use: the terminal still works.
                        Err(e) if std::env::var_os("WOLE_LINK_DEBUG").is_some() => {
                            console::attach();
                            eprintln!("window: {e}");
                        }
                        Err(_) => {}
                    }
                }
                // Opened with a double click on Windows there's no terminal to tell this to.
                None if args.card.is_none() => {
                    if gui::message(&text(lang, Msg::NoCard)).is_ok() {
                        return ExitCode::FAILURE;
                    }
                }
                None => {}
            }
        }
    }

    // The console program already has its own console and standard handles.
    if !cli {
        console::attach();
    }
    let ansi = std::io::stdout().is_terminal() && console::enable_ansi();
    let args = match args {
        Ok(args) => args,
        Err(bad) => {
            eprintln!("{bad}?\n\n{}", usage(lang, cli));
            return ExitCode::from(2);
        }
    };
    if args.help {
        println!("{}", usage(lang, cli));
        return ExitCode::SUCCESS;
    }
    let chosen = match &args.card {
        Some(name) => match cards.iter().position(|c| &c.name == name) {
            None => {
                eprintln!("{}", text(lang, Msg::CardNotFound(name)));
                print_cards(lang, &cards, None, true);
                return ExitCode::FAILURE;
            }
            Some(i) if !cards[i].usable() => {
                let msg = match cards[i].kind {
                    Kind::Virtual | Kind::Loopback => Msg::CardVirtual(name),
                    _ => Msg::CardNotUsable(name),
                };
                eprintln!("{}", text(lang, msg));
                return ExitCode::FAILURE;
            }
            Some(i) => Some(i),
        },
        None => net::suggest(&cards),
    };
    print_cards(lang, &cards, chosen, args.list_only);
    if args.list_only {
        return ExitCode::SUCCESS;
    }
    let Some(card) = chosen.map(|i| &cards[i]) else {
        eprintln!("\n{}", text(lang, Msg::NoCard));
        return ExitCode::FAILURE;
    };
    if card.kind == Kind::Wireless && args.card.is_none() {
        println!("\n{}", text(lang, Msg::WirelessWarning));
    }

    let url = link::build(&ui::device(card, hostname.as_deref()));
    println!("\n{}\n", text(lang, Msg::Scan));
    print!("{}", qr::render(&url, ansi));
    println!("\n{url}");
    ExitCode::SUCCESS
}

/// The options, under the name of the program that was run. On Windows the window program points
/// to the console one, which is the one that behaves in a terminal.
fn usage(lang: Lang, cli: bool) -> String {
    let usage = text(lang, Msg::Usage(if cli { "wole-link-cli" } else { "wole-link" }));
    if cfg!(windows) && !cli {
        format!("{usage}\n\n{}", text(lang, Msg::UseCli))
    } else {
        usage
    }
}

/// `>` marks the card in the QR. Virtual and loopback cards only show with `all` (`--list`, or a `--card` that isn't found).
fn print_cards(lang: Lang, cards: &[Card], chosen: Option<usize>, all: bool) {
    let suggested = net::suggest(cards);
    println!("{}:", text(lang, Msg::Cards));
    for (i, card) in cards.iter().enumerate() {
        if !all && matches!(card.kind, Kind::Virtual | Kind::Loopback) {
            continue;
        }
        let kind = text(
            lang,
            match card.kind {
                Kind::Wired => Msg::Wired,
                Kind::Wireless => Msg::Wireless,
                Kind::Virtual => Msg::Virtual,
                Kind::Loopback => Msg::Loopback,
            },
        );
        let mut line = format!(
            "{} {}{}  [{kind}]",
            if Some(i) == chosen { '>' } else { ' ' },
            card.name,
            card.description.as_deref().map(|d| format!(" ({d})")).unwrap_or_default(),
        );
        if let Some(mac) = card.mac.filter(|m| *m != [0; 6]) {
            line += &format!("  {}", link::format_mac(&mac));
        }
        match (card.up, card.ipv4) {
            (false, _) => line += &format!("  {}", text(lang, Msg::Disconnected)),
            (true, Some((ip, prefix))) => line += &format!("  {ip}/{prefix}"),
            (true, None) => line += &format!("  {}", text(lang, Msg::NoIpv4)),
        }
        if Some(i) == suggested {
            line += &format!("  ({})", text(lang, Msg::Suggested));
        }
        println!("{line}");
    }
}

#[cfg(unix)]
mod console {
    pub fn attach() {}

    pub fn enable_ansi() -> bool {
        std::env::var_os("NO_COLOR").is_none()
    }

    pub fn owns_window() -> bool {
        false
    }

    pub fn restore() {}
}

#[cfg(windows)]
mod console {
    use std::ptr::null;
    use std::sync::Mutex;
    use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Console::{
        AllocConsole, AttachConsole, GetConsoleMode, GetConsoleOutputCP, GetConsoleProcessList,
        GetStdHandle, SetConsoleMode, SetConsoleOutputCP, SetStdHandle, ATTACH_PARENT_PROCESS,
        ENABLE_VIRTUAL_TERMINAL_PROCESSING, STD_ERROR_HANDLE, STD_INPUT_HANDLE,
        STD_OUTPUT_HANDLE,
    };

    /// The code page and output mode found in a borrowed console, put back on exit.
    static SAVED: Mutex<Option<(u32, u32)>> = Mutex::new(None);

    /// Borrows the console of the `cmd` or PowerShell we were started from, or opens one (a
    /// double click that ended up with nothing to show in the window). Handles that already
    /// point somewhere, like `--list > cards.txt`, are kept.
    pub fn attach() {
        // SAFETY: console and handle calls on this process's own standard handles.
        unsafe {
            let handles = [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE, STD_INPUT_HANDLE];
            // What `cmd` or PowerShell handed us (a file, a pipe) or nothing.
            let given = handles.map(|which| {
                let h = GetStdHandle(which);
                (!h.is_null() && h != INVALID_HANDLE_VALUE).then_some(h)
            });
            if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
                AllocConsole();
            }
            let open = |name: &str| {
                let wide: Vec<u16> = name.encode_utf16().chain([0]).collect();
                CreateFileW(
                    wide.as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    null(),
                    OPEN_EXISTING,
                    0,
                    std::ptr::null_mut(),
                )
            };
            // Attaching can point the standard handles at the console, `> cards.txt` included:
            // put back what we were given, and the console only where there was nothing.
            for (which, given) in handles.into_iter().zip(given) {
                let console = if which == STD_INPUT_HANDLE { "CONIN$" } else { "CONOUT$" };
                SetStdHandle(which, given.unwrap_or_else(|| open(console)));
            }
            if std::env::var_os("WOLE_LINK_DEBUG").is_some() {
                let [out, err, inp] = given.map(|h| h.is_some());
                eprintln!("handles given: stdout {out}, stderr {err}, stdin {inp}");
            }
        }
    }

    /// UTF-8 output for the blocks and accents, and escape codes for the colors.
    pub fn enable_ansi() -> bool {
        // SAFETY: plain console calls on this process's own standard output.
        unsafe {
            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            let mut mode = 0;
            if GetConsoleMode(handle, &mut mode) == 0 {
                return false;
            }
            *SAVED.lock().unwrap() = Some((GetConsoleOutputCP(), mode));
            SetConsoleOutputCP(65001);
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
                && std::env::var_os("NO_COLOR").is_none()
        }
    }

    /// Leaves the user's console as we found it.
    pub fn restore() {
        if let Some((code_page, mode)) = SAVED.lock().unwrap().take() {
            // SAFETY: as in `enable_ansi`.
            unsafe {
                SetConsoleOutputCP(code_page);
                SetConsoleMode(GetStdHandle(STD_OUTPUT_HANDLE), mode);
            }
        }
    }

    /// The console was opened for us (a double click): it closes, text and all, when we exit.
    pub fn owns_window() -> bool {
        let mut ids = [0u32; 2];
        // SAFETY: the count passed is the array's length.
        unsafe { GetConsoleProcessList(ids.as_mut_ptr(), ids.len() as u32) == 1 }
    }
}
