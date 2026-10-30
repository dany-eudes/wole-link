//! Linux: cards from /sys/class/net, IPv4 from getifaddrs, default route from /proc/net/route.

use super::{looks_virtual, Card, Kind};
use std::collections::HashMap;
use std::ffi::CStr;
use std::fs;
use std::net::Ipv4Addr;
use std::path::Path;

pub fn list() -> Vec<Card> {
    let addresses = ipv4_addresses();
    let default_routes = default_route_interfaces();
    let Ok(entries) = fs::read_dir("/sys/class/net") else { return Vec::new() };
    let mut cards: Vec<(u32, Option<String>, Card)> = entries
        .flatten()
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let dir = entry.path();
            let index = read(&dir, "ifindex").and_then(|s| s.parse().ok()).unwrap_or(u32::MAX);
            let master = fs::read_link(dir.join("master"))
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()));
            let card = Card {
                kind: kind(&name, &dir),
                // "unknown" is what some drivers report while working; the carrier says more.
                up: match read(&dir, "operstate").as_deref() {
                    Some("up") => true,
                    Some("unknown") => read(&dir, "carrier").as_deref() == Some("1"),
                    _ => false,
                },
                mac: read(&dir, "address").and_then(|s| parse_mac(&s)),
                ipv4: addresses.get(&name).copied(),
                default_route: default_routes.contains(&name),
                description: None,
                name,
            };
            (index, master, card)
        })
        .collect();
    cards.sort_by_key(|(index, _, _)| *index);
    let (masters, mut cards): (Vec<_>, Vec<_>) = cards.into_iter().map(|(_, m, c)| (m, c)).unzip();
    inherit_from_master(&mut cards, &masters);
    cards
}

/// A wired card in a bridge or bond (`enp3s0` in `br0` on a Proxmox or KVM host) has no address
/// of its own: the master holds it. The card keeps its MAC, the one the magic packet must carry,
/// and takes the master's IPv4 and default route.
fn inherit_from_master(cards: &mut [Card], masters: &[Option<String>]) {
    for i in 0..cards.len() {
        let Some(master) = &masters[i] else { continue };
        if cards[i].kind != Kind::Wired || cards[i].ipv4.is_some() {
            continue;
        }
        if let Some(m) = cards.iter().position(|c| &c.name == master) {
            let (ipv4, default_route) = (cards[m].ipv4, cards[m].default_route);
            cards[i].ipv4 = ipv4;
            cards[i].default_route |= default_route;
        }
    }
}

pub fn hostname() -> Option<String> {
    read(Path::new("/proc/sys/kernel"), "hostname").filter(|h| !h.is_empty())
}

fn read(dir: &Path, file: &str) -> Option<String> {
    fs::read_to_string(dir.join(file)).ok().map(|s| s.trim().to_string())
}

fn kind(name: &str, dir: &Path) -> Kind {
    // ARPHRD_LOOPBACK
    if name == "lo" || read(dir, "type").as_deref() == Some("772") {
        Kind::Loopback
    } else if dir.join("wireless").exists() || dir.join("phy80211").exists() {
        Kind::Wireless
    } else if !dir.join("device").exists() || looks_virtual(name, None) {
        // Bridges, veth pairs, tunnels and the like have no hardware device behind them.
        Kind::Virtual
    } else {
        Kind::Wired
    }
}

fn parse_mac(text: &str) -> Option<[u8; 6]> {
    let parts: Vec<u8> =
        text.split(':').map(|p| u8::from_str_radix(p, 16)).collect::<Result<_, _>>().ok()?;
    parts.try_into().ok()
}

/// First IPv4 address and prefix length of each interface.
fn ipv4_addresses() -> HashMap<String, (Ipv4Addr, u8)> {
    let mut found = HashMap::new();
    let mut head: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: getifaddrs fills `head` with a list we walk read-only and free once below.
    unsafe {
        if libc::getifaddrs(&mut head) != 0 {
            return found;
        }
        let mut cursor = head;
        while let Some(entry) = cursor.as_ref() {
            cursor = entry.ifa_next;
            let (addr, mask) = (entry.ifa_addr, entry.ifa_netmask);
            if addr.is_null() || mask.is_null() || (*addr).sa_family as i32 != libc::AF_INET {
                continue;
            }
            let ip = (*(addr as *const libc::sockaddr_in)).sin_addr.s_addr;
            let mask = (*(mask as *const libc::sockaddr_in)).sin_addr.s_addr;
            let name = CStr::from_ptr(entry.ifa_name).to_string_lossy().into_owned();
            found
                .entry(name)
                .or_insert((Ipv4Addr::from(u32::from_be(ip)), u32::from_be(mask).count_ones() as u8));
        }
        libc::freeifaddrs(head);
    }
    found
}

/// Interfaces with a 0.0.0.0/0 route.
fn default_route_interfaces() -> Vec<String> {
    let Ok(table) = fs::read_to_string("/proc/net/route") else { return Vec::new() };
    parse_default_routes(&table)
}

fn parse_default_routes(table: &str) -> Vec<String> {
    table
        .lines()
        .skip(1)
        .filter_map(|line| {
            let cols: Vec<&str> = line.split_whitespace().collect();
            (cols.len() > 7 && cols[1] == "00000000" && cols[7] == "00000000")
                .then(|| cols[0].to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(name: &str, kind: Kind, ipv4: Option<(Ipv4Addr, u8)>, default_route: bool) -> Card {
        Card {
            name: name.into(),
            description: None,
            kind,
            up: true,
            mac: Some([2, 0, 0, 0, 0, 1]),
            ipv4,
            default_route,
        }
    }

    #[test]
    fn bridged_wired_card_takes_the_bridge_address() {
        let ip = Some((Ipv4Addr::new(192, 168, 0, 5), 24));
        let mut cards = vec![
            card("enp3s0", Kind::Wired, None, false),
            card("br0", Kind::Virtual, ip, true),
            card("veth1", Kind::Virtual, None, false),
        ];
        inherit_from_master(&mut cards, &[Some("br0".into()), None, Some("br0".into())]);
        assert_eq!((cards[0].ipv4, cards[0].default_route), (ip, true));
        assert_eq!(cards[0].mac, Some([2, 0, 0, 0, 0, 1]));
        // Only wired cards inherit: the veth stays as it was.
        assert_eq!(cards[2].ipv4, None);
        assert_eq!(super::super::suggest(&cards), Some(0));
    }

    #[test]
    fn mac_from_sysfs() {
        assert_eq!(parse_mac("a8:5e:45:0d:ee:01"), Some([0xa8, 0x5e, 0x45, 0x0d, 0xee, 0x01]));
        assert_eq!(parse_mac("00:00:00:00"), None);
        assert_eq!(parse_mac(""), None);
    }

    #[test]
    fn default_routes_from_proc() {
        let table = "\
Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
enp3s0\t00000000\t0100A8C0\t0003\t0\t0\t100\t00000000\t0\t0\t0
enp3s0\t0000A8C0\t00000000\t0001\t0\t0\t100\t00FFFFFF\t0\t0\t0
docker0\t000011AC\t00000000\t0001\t0\t0\t0\t0000FFFF\t0\t0\t0
";
        assert_eq!(parse_default_routes(table), vec!["enp3s0"]);
    }
}
