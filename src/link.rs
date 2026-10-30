//! The registration link, format v1. See "Link format" in README.md: the app, the site's /add
//! page and this utility all read and write exactly this.

use std::net::Ipv4Addr;

pub const BASE: &str = "https://wole.celerasoft.com/add";
pub const VERSION: u32 = 1;
/// A hostname label is at most 63 bytes; longer names are cut there so the QR stays small.
pub const NAME_MAX_BYTES: usize = 63;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub mac: [u8; 6],
    pub name: Option<String>,
    pub ip: Option<Ipv4Addr>,
    pub broadcast: Option<Ipv4Addr>,
    pub port: Option<u16>,
}

/// `AA-BB-CC-DD-EE-FF`, the way the app shows a MAC.
pub fn format_mac(mac: &[u8; 6]) -> String {
    mac.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join("-")
}

/// Everything but RFC 3986's unreserved characters is percent-encoded, byte by byte (UTF-8).
fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn truncate_utf8(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// The data goes after `#`, so it never reaches the server. Fields keep a fixed order and absent
/// ones are left out; the app falls back to its own defaults for them.
pub fn build(device: &Device) -> String {
    let mut fields = vec![format!("v={VERSION}"), format!("mac={}", format_mac(&device.mac))];
    if let Some(ip) = device.ip {
        fields.push(format!("ip={ip}"));
    }
    if let Some(bc) = device.broadcast {
        fields.push(format!("bc={bc}"));
    }
    if let Some(port) = device.port {
        fields.push(format!("port={port}"));
    }
    if let Some(name) = device.name.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        fields.push(format!("name={}", encode(truncate_utf8(name, NAME_MAX_BYTES))));
    }
    format!("{BASE}#{}", fields.join("&"))
}

/// Directed broadcast of `ip` inside a network of `prefix` bits.
pub fn broadcast(ip: Ipv4Addr, prefix: u8) -> Ipv4Addr {
    let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix.min(32) as u32) };
    Ipv4Addr::from(u32::from(ip) | !mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAC: [u8; 6] = [0xaa, 0xbb, 0xcc, 0x0d, 0xee, 0x01];

    fn device() -> Device {
        Device { mac: MAC, name: None, ip: None, broadcast: None, port: None }
    }

    #[test]
    fn mac_only() {
        assert_eq!(build(&device()), "https://wole.celerasoft.com/add#v=1&mac=AA-BB-CC-0D-EE-01");
    }

    #[test]
    fn all_fields_in_order() {
        let d = Device {
            name: Some("DESKTOP-01".into()),
            ip: Some(Ipv4Addr::new(192, 168, 0, 20)),
            broadcast: Some(Ipv4Addr::new(192, 168, 0, 255)),
            port: Some(9),
            ..device()
        };
        assert_eq!(
            build(&d),
            "https://wole.celerasoft.com/add#v=1&mac=AA-BB-CC-0D-EE-01&ip=192.168.0.20\
             &bc=192.168.0.255&port=9&name=DESKTOP-01"
        );
    }

    #[test]
    fn name_is_percent_encoded_utf8() {
        let d = Device { name: Some("Sala & João".into()), ..device() };
        assert!(build(&d).ends_with("&name=Sala%20%26%20Jo%C3%A3o"));
    }

    #[test]
    fn blank_name_is_left_out() {
        let d = Device { name: Some("   ".into()), ..device() };
        assert!(!build(&d).contains("name="));
    }

    #[test]
    fn long_name_is_cut_on_a_char_boundary() {
        let d = Device { name: Some("ç".repeat(40)), ..device() };
        let link = build(&d);
        let encoded = link.split("name=").nth(1).unwrap();
        // 31 "ç" = 62 bytes; a 32nd would pass 63.
        assert_eq!(encoded, "%C3%A7".repeat(31));
    }

    #[test]
    fn broadcast_of_common_prefixes() {
        let ip = Ipv4Addr::new(10, 200, 50, 37);
        assert_eq!(broadcast(ip, 24), Ipv4Addr::new(10, 200, 50, 255));
        assert_eq!(broadcast(ip, 23), Ipv4Addr::new(10, 200, 51, 255));
        assert_eq!(broadcast(ip, 32), ip);
        assert_eq!(broadcast(ip, 0), Ipv4Addr::BROADCAST);
    }
}
