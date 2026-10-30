//! Network cards and the rule that picks the one to register.

use std::net::Ipv4Addr;

#[cfg(unix)]
mod linux;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use linux::{hostname, list};
#[cfg(windows)]
pub use windows::{hostname, list};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Wired,
    Wireless,
    /// Docker, bridges, VPN, Hyper-V and other cards with no hardware behind them.
    Virtual,
    Loopback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    /// What the user picks with `--card`: `enp3s0` on Linux, "Ethernet" on Windows.
    pub name: String,
    /// Longer text when the system has one (Windows adapter description).
    pub description: Option<String>,
    pub kind: Kind,
    pub up: bool,
    pub mac: Option<[u8; 6]>,
    /// Address and prefix length.
    pub ipv4: Option<(Ipv4Addr, u8)>,
    pub default_route: bool,
}

/// Name prefixes of virtual cards, for when the system doesn't say so itself.
const VIRTUAL_PREFIXES: &[&str] = &[
    "docker", "br-", "veth", "virbr", "vEthernet", "tun", "tap", "wg", "tailscale", "zt", "vmnet",
    "vboxnet", "lxc", "lxd", "cni", "flannel", "podman", "ppp",
];
/// Words that mark a Windows adapter description as virtual.
const VIRTUAL_WORDS: &[&str] = &[
    "virtual", "hyper-v", "vpn", "tap-", "wireguard", "tailscale", "vmware", "virtualbox",
    "loopback", "bluetooth", "wan miniport", "zerotier", "openvpn", "npcap",
];

pub fn looks_virtual(name: &str, description: Option<&str>) -> bool {
    let lower = name.to_ascii_lowercase();
    VIRTUAL_PREFIXES.iter().any(|p| lower.starts_with(&p.to_ascii_lowercase()))
        || description.is_some_and(|d| {
            let d = d.to_ascii_lowercase();
            VIRTUAL_WORDS.iter().any(|w| d.contains(w))
        })
}

impl Card {
    /// Up, with a real MAC and an IPv4 address, and backed by hardware.
    pub fn usable(&self) -> bool {
        self.up
            && self.ipv4.is_some()
            && self.mac.is_some_and(|m| m != [0; 6])
            && matches!(self.kind, Kind::Wired | Kind::Wireless)
    }
}

/// Index of the card to preselect: a usable wired card, the one on the default route first;
/// failing that, a usable wireless one (Wake-on-LAN rarely works over Wi-Fi, so the caller
/// warns). Ties keep the system's order.
pub fn suggest(cards: &[Card]) -> Option<usize> {
    let rank = |c: &Card| match (c.kind, c.default_route) {
        (Kind::Wired, true) => 0,
        (Kind::Wired, false) => 1,
        (_, true) => 2,
        _ => 3,
    };
    cards
        .iter()
        .enumerate()
        .filter(|(_, c)| c.usable())
        .min_by_key(|(i, c)| (rank(c), *i))
        .map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(name: &str, kind: Kind, default_route: bool) -> Card {
        Card {
            name: name.into(),
            description: None,
            kind,
            up: true,
            mac: Some([0x10, 0x20, 0x30, 0x40, 0x50, 0x60]),
            ipv4: Some((Ipv4Addr::new(192, 168, 0, 20), 24)),
            default_route,
        }
    }

    #[test]
    fn wired_beats_wireless_on_the_default_route() {
        // The user's desktop: Wi-Fi holds the default route, the cable is still the one to wake.
        let cards = [
            card("lo", Kind::Loopback, false),
            card("wlp4s0", Kind::Wireless, true),
            card("docker0", Kind::Virtual, false),
            card("enp3s0", Kind::Wired, false),
        ];
        assert_eq!(suggest(&cards), Some(3));
    }

    #[test]
    fn default_route_breaks_a_tie_between_wired_cards() {
        let cards = [card("enp1s0", Kind::Wired, false), card("enp2s0", Kind::Wired, true)];
        assert_eq!(suggest(&cards), Some(1));
    }

    #[test]
    fn wireless_only_when_there_is_no_wired_card() {
        let cards = [card("wlan0", Kind::Wireless, true), card("tailscale0", Kind::Virtual, false)];
        assert_eq!(suggest(&cards), Some(0));
    }

    #[test]
    fn down_cards_and_cards_without_ipv4_or_mac_are_skipped() {
        let mut down = card("enp1s0", Kind::Wired, false);
        down.up = false;
        let mut no_ip = card("enp2s0", Kind::Wired, false);
        no_ip.ipv4 = None;
        let mut zero_mac = card("enp3s0", Kind::Wired, false);
        zero_mac.mac = Some([0; 6]);
        assert_eq!(suggest(&[down, no_ip, zero_mac]), None);
    }

    #[test]
    fn virtual_names_and_descriptions() {
        assert!(looks_virtual("br-3f2a1b", None));
        assert!(looks_virtual("vEthernet (WSL)", None));
        assert!(looks_virtual("Ethernet 3", Some("TAP-Windows Adapter V9")));
        assert!(looks_virtual("Ethernet 2", Some("VirtualBox Host-Only Ethernet Adapter")));
        assert!(!looks_virtual("enp3s0", None));
        assert!(!looks_virtual("Ethernet", Some("Realtek PCIe GbE Family Controller")));
    }
}
