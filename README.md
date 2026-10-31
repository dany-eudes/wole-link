# wole-link

Portable, open-source utility that finds your computer's wired network card and shows a QR code
to register it in WOL•E, the Wake-on-LAN app for Android. Single file, nothing to install, for
Windows and Linux.

Download page: https://wole.celerasoft.com/wole-link.html · [Code signing policy](#code-signing-policy)

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

The ☰ menu has the language (remembered in `~/.config/wole-link/settings.ini` or
`%APPDATA%\wole-link\settings.ini`), About, links to the site, Google Play, Buy Me a Coffee and
support, and on Linux "Add to the applications menu": GNOME only shows a program's icon in the
dock when it has a `.desktop` entry, so this writes one (and the icon) under `~/.local/share`,
and removes them again from the same menu.

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

On Windows there are two programs, built from the same code:

- `wole-link.exe`, a window program: a double click opens only the window, with no console
  flashing up. Started from `cmd` or PowerShell with `--text` or `--list` it still writes to that
  console, but the prompt comes back before the output (Windows doesn't wait for window
  programs), and in PowerShell `> file.txt` stays empty.
- `wole-link-cli.exe`, a console program for the terminal: the QR (or `--list`) in the console,
  never the window. The prompt waits for it and `>` works, in PowerShell too.

On Linux, `wole-link` does both: the window when there's a display, the terminal with `--text`,
`--list` or without a display.

## Removing it

There is nothing to uninstall: delete the program file. WOL•E Link changes nothing on the system
except two things, both under your user account:

- the language you pick in the ☰ menu, in `%APPDATA%\wole-link\settings.ini` (Windows) or
  `~/.config/wole-link/settings.ini` (Linux); delete the `wole-link` folder to remove it;
- on Linux, only if you choose "Add to the applications menu", `wole-link.desktop` and its icons
  under `~/.local/share`; "Remove from the applications menu", in the same menu, deletes them.

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

## Checking a download

Each release has `SHA256SUMS` and its [minisign](https://jedisct1.github.io/minisign/)
signature, `SHA256SUMS.minisig`. The public key:

```
RWRFM/lCBXDabL4fkMuiqfVcYEZxApAOgje+fpObZbcMwOmM0Tr5VRm7
```

```
minisign -Vm SHA256SUMS -P RWRFM/lCBXDabL4fkMuiqfVcYEZxApAOgje+fpObZbcMwOmM0Tr5VRm7
sha256sum -c --ignore-missing SHA256SUMS
```

The first line proves the list of files comes from the author; the second, that the files you
downloaded are the ones on the list.

## Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io/), certificate by
[SignPath Foundation](https://signpath.org/).

The Windows programs (`wole-link.exe` and `wole-link-cli.exe`) are signed from the first signed
release on; Windows shows "SignPath Foundation" as the publisher. Every signed file is built by
GitHub Actions, on GitHub's own runners, from the source code of a release tag in this
repository ([build.yml](.github/workflows/build.yml)); nothing built elsewhere is signed, and
each signing request is approved by hand.

Team roles:

- Committers and reviewers: [Dany Eudes](https://github.com/dany-eudes)
- Approvers: [Dany Eudes](https://github.com/dany-eudes)

Privacy policy: this program will not transfer any information to other networked systems unless
specifically requested by the user or the person installing or operating it. It reads the
network cards and the computer's name only to draw the QR code on the screen; the menu's links
open in the browser only when you click them.

## Building

Everything runs in Docker; nothing to install on the host besides Docker and `make`.

```
make test      # unit tests
make linux     # target/x86_64-unknown-linux-musl/release/wole-link (static)
make windows   # target/x86_64-pc-windows-gnu/release/wole-link.exe and wole-link-cli.exe (VM tests)
make windows32 # 32-bit build, only for testing on a 32-bit VM (Windows 10 or later)
make png       # the window in each language, as target/ui-*.png
```

## Icon

The WOL•E app's icon with its second bar ending in a QR finder pattern. `python3 tools/icons.py`
draws every size from the app's own curves (16 and 32 px on the pixel grid) and writes the
Windows `.ico` and the X11 icon in `assets/icon/`.

## License

[MIT](LICENSE), © 2026 @celeraman: the code and everything in this repository, the icon included.
The embedded font is Noto Sans, subset to the Latin characters the four languages use, under the
[SIL Open Font License 1.1](assets/fonts/OFL.txt).

"WOL•E", "WOL•E Link" and the WOL•E logo are the author's trademarks. The MIT License covers the
code and the files, not the names: a modified version must not be distributed under these names
or with this icon in a way that suggests it is the original.
