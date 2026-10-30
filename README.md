# wole-link

Portable, open-source utility that finds your computer's wired network card and shows a QR code
to register it in WOL•E, the Wake-on-LAN app for Android. Single file, nothing to install, for
Windows and Linux.

> Work in progress. The display name and the visual identity are not final yet.

## What it does

1. Lists the network cards and preselects the one Wake-on-LAN is most likely to work with: up,
   with an IPv4 address, on the default route, and wired. Loopback, Docker (`docker0`, `br-`,
   `veth`), Hyper-V (`vEthernet`), VPN and other virtual cards are left out.
2. Shows a QR code with the registration link for that card: MAC, IP, broadcast, port and the
   computer's name.
3. You scan it with the phone. With WOL•E installed, the link opens the app's Add device form
   already filled in; without it, it opens the site, which links to the Play Store.

Native window on Windows (Win32) and on Linux (X11, also under Wayland through XWayland), with an
embedded font. With no display, it prints the QR in the terminal.

The window follows the screen's scale: `Xft.dpi` (and `GDK_SCALE`) on Linux, the display
scale setting on Windows. `WOLE_LINK_SCALE=1.5` overrides it.

Languages: English, Portuguese (Brazil), Portuguese (Portugal) and Spanish, the same as the app.

## Usage

```
wole-link                 # window with the suggested card; click another card to switch
wole-link --text          # QR in the terminal (also when there's no display)
wole-link --card NAME     # another card (the name shown in the list)
wole-link --list          # every card, virtual ones included
wole-link --lang pt-BR    # en, pt-BR, pt-PT or es; default: the system's language
```

On Windows, `wole-link.exe` is a window program: a double click opens only the window. Started
from `cmd` or PowerShell with `--text` or `--list`, it writes to that console, but the prompt
comes back before the output (Windows doesn't wait for window programs).

## Link format (v1)

```
https://wole.celerasoft.com/add#v=1&mac=AA-BB-CC-DD-EE-FF&ip=192.168.0.20&bc=192.168.0.255&port=9&name=DESKTOP-01
```

| Key    | Value                                             | Required |
|--------|---------------------------------------------------|----------|
| `v`    | Format version, `1`                               | yes      |
| `mac`  | MAC address, upper case, `-` separated            | yes      |
| `ip`   | IPv4 address of the card (status check host)      | no       |
| `bc`   | Directed broadcast address of the card's network  | no       |
| `port` | Wake-on-LAN UDP port                              | no       |
| `name` | Device name, UTF-8, percent-encoded, ≤ 63 bytes   | no       |

- The data goes after `#`, so it is never sent to the server.
- Keys come in this order; keys without a value are left out, and the app uses its own defaults
  for them.
- Readers ignore keys they don't know, so later versions can add fields.

## Building

Everything runs in Docker; nothing to install on the host besides Docker and `make`.

```
make test      # unit tests
make linux     # target/x86_64-unknown-linux-musl/release/wole-link (static)
make windows   # target/x86_64-pc-windows-gnu/release/wole-link.exe (for testing in a VM)
make windows32 # 32-bit build, only for testing on a 32-bit VM (Windows 10 or later)
make png       # the window in each language, as target/ui-*.png
```

## License

[MIT](LICENSE). The embedded font is Noto Sans, subset to the Latin characters the four
languages use, under the [SIL Open Font License 1.1](assets/fonts/OFL.txt).
