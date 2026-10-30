mod i18n;
mod link;
mod net;
mod qr;

use i18n::{text, Lang, Msg};
use net::{Card, Kind};
use std::io::IsTerminal;
use std::process::ExitCode;

struct Args {
    card: Option<String>,
    list_only: bool,
    lang: Option<Lang>,
    help: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args { card: None, list_only: false, lang: None, help: false };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--card" => args.card = Some(it.next().ok_or("--card")?),
            "--list" => args.list_only = true,
            "--lang" => args.lang = Some(i18n::parse(&it.next().ok_or("--lang")?)),
            "-h" | "--help" => args.help = true,
            other => return Err(other.to_string()),
        }
    }
    Ok(args)
}

fn main() -> ExitCode {
    let code = run();
    if console::owns_window() {
        let lang = parse_args().ok().and_then(|a| a.lang).unwrap_or_else(i18n::detect);
        println!("\n{}", text(lang, Msg::PressEnter));
        let _ = std::io::stdin().read_line(&mut String::new());
    }
    code
}

fn run() -> ExitCode {
    let ansi = std::io::stdout().is_terminal() && console::enable_ansi();
    let args = match parse_args() {
        Ok(args) => args,
        Err(bad) => {
            eprintln!("{bad}?\n\n{}", text(i18n::detect(), Msg::Usage));
            return ExitCode::from(2);
        }
    };
    let lang = args.lang.unwrap_or_else(i18n::detect);
    if args.help {
        println!("{}", text(lang, Msg::Usage));
        return ExitCode::SUCCESS;
    }

    let cards = net::list();
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

    let (ip, prefix) = card.ipv4.expect("usable cards have IPv4");
    let device = link::Device {
        mac: card.mac.expect("usable cards have a MAC"),
        name: net::hostname(),
        ip: Some(ip),
        broadcast: Some(link::broadcast(ip, prefix)),
        port: Some(9),
    };
    let url = link::build(&device);
    println!("\n{}\n", text(lang, Msg::Scan));
    print!("{}", qr::render(&url, ansi));
    println!("\n{url}");
    ExitCode::SUCCESS
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
    pub fn enable_ansi() -> bool {
        std::env::var_os("NO_COLOR").is_none()
    }

    pub fn owns_window() -> bool {
        false
    }
}

#[cfg(windows)]
mod console {
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetConsoleProcessList, GetStdHandle, SetConsoleMode, SetConsoleOutputCP,
        ENABLE_VIRTUAL_TERMINAL_PROCESSING, STD_OUTPUT_HANDLE,
    };

    /// UTF-8 output for the blocks and accents, and escape codes for the colors.
    pub fn enable_ansi() -> bool {
        // SAFETY: plain console calls on this process's own standard output.
        unsafe {
            SetConsoleOutputCP(65001);
            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            let mut mode = 0;
            GetConsoleMode(handle, &mut mode) != 0
                && SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
                && std::env::var_os("NO_COLOR").is_none()
        }
    }

    /// Started by a double click in Explorer: the console window is ours alone and would close
    /// with the QR the moment we exit.
    pub fn owns_window() -> bool {
        let mut ids = [0u32; 2];
        // SAFETY: the count passed is the array's length.
        unsafe { GetConsoleProcessList(ids.as_mut_ptr(), ids.len() as u32) == 1 }
    }
}
