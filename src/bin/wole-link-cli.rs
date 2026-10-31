// The terminal program: the QR (or `--list`) in the console it was started from, never the
// window. A console program on Windows, so the prompt waits for it and `>` works in PowerShell.

fn main() -> std::process::ExitCode {
    wole_link_app::main(true)
}
