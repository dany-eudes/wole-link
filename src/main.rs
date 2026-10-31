// The window. A window program on Windows: no console flashes up on a double click, and
// `--text`/`--list` borrow the terminal it was started from. In a terminal, `wole-link-cli`.
#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    wole_link_app::main(false)
}
