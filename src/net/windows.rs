//! Windows: everything from GetAdaptersAddresses. An adapter with a gateway stands in for the
//! default route.

use super::{looks_virtual, Card, Kind};
use std::net::Ipv4Addr;
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GetAdaptersAddresses, GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_SKIP_ANYCAST,
    GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST, IF_TYPE_IEEE80211, IF_TYPE_PROP_VIRTUAL,
    IF_TYPE_SOFTWARE_LOOPBACK, IF_TYPE_TUNNEL, IP_ADAPTER_ADDRESSES_LH,
};
use windows_sys::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows_sys::Win32::Networking::WinSock::{AF_INET, SOCKADDR_IN};
use windows_sys::Win32::System::SystemInformation::{
    ComputerNameDnsHostname, GetComputerNameExW,
};

const ERROR_BUFFER_OVERFLOW: u32 = 111;

pub fn list() -> Vec<Card> {
    let flags = GAA_FLAG_INCLUDE_GATEWAYS
        | GAA_FLAG_SKIP_ANYCAST
        | GAA_FLAG_SKIP_MULTICAST
        | GAA_FLAG_SKIP_DNS_SERVER;
    // Microsoft suggests starting at 15 KB; retry with the size it asks for.
    let mut size: u32 = 15 * 1024;
    let mut buffer: Vec<u64> = Vec::new();
    for _ in 0..3 {
        buffer = vec![0; (size as usize).div_ceil(8)];
        // SAFETY: the buffer is `size` bytes, 8-aligned as the structures need.
        let result = unsafe {
            GetAdaptersAddresses(
                AF_INET as u32,
                flags,
                std::ptr::null(),
                buffer.as_mut_ptr().cast(),
                &mut size,
            )
        };
        match result {
            0 => break,
            ERROR_BUFFER_OVERFLOW => continue,
            _ => return Vec::new(),
        }
    }

    let mut cards = Vec::new();
    let mut cursor = buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
    // SAFETY: a linked list inside `buffer`, filled by GetAdaptersAddresses above.
    while let Some(adapter) = unsafe { cursor.as_ref() } {
        cursor = adapter.Next;
        let name = unsafe { wide(adapter.FriendlyName) };
        let description = Some(unsafe { wide(adapter.Description) }).filter(|d| !d.is_empty());
        let kind = match adapter.IfType {
            IF_TYPE_SOFTWARE_LOOPBACK => Kind::Loopback,
            IF_TYPE_PROP_VIRTUAL | IF_TYPE_TUNNEL => Kind::Virtual,
            _ if looks_virtual(&name, description.as_deref()) => Kind::Virtual,
            IF_TYPE_IEEE80211 => Kind::Wireless,
            _ => Kind::Wired,
        };
        let mac = (adapter.PhysicalAddressLength == 6).then(|| {
            let mut mac = [0; 6];
            mac.copy_from_slice(&adapter.PhysicalAddress[..6]);
            mac
        });
        cards.push(Card {
            name,
            description,
            kind,
            up: adapter.OperStatus == IfOperStatusUp,
            mac,
            ipv4: unsafe { first_ipv4(adapter) },
            default_route: !adapter.FirstGatewayAddress.is_null(),
        });
    }
    cards
}

pub fn hostname() -> Option<String> {
    let mut buffer = [0u16; 256];
    let mut size = buffer.len() as u32;
    // SAFETY: `size` is the buffer's length in UTF-16 units.
    let ok = unsafe { GetComputerNameExW(ComputerNameDnsHostname, buffer.as_mut_ptr(), &mut size) };
    (ok != 0 && size > 0).then(|| String::from_utf16_lossy(&buffer[..size as usize]))
}

unsafe fn first_ipv4(adapter: &IP_ADAPTER_ADDRESSES_LH) -> Option<(Ipv4Addr, u8)> {
    let mut cursor = adapter.FirstUnicastAddress;
    while let Some(unicast) = cursor.as_ref() {
        cursor = unicast.Next;
        let sockaddr = unicast.Address.lpSockaddr;
        if !sockaddr.is_null() && (*sockaddr).sa_family == AF_INET {
            let raw = (*(sockaddr as *const SOCKADDR_IN)).sin_addr.S_un.S_addr;
            return Some((Ipv4Addr::from(u32::from_be(raw)), unicast.OnLinkPrefixLength));
        }
    }
    None
}

unsafe fn wide(text: *const u16) -> String {
    if text.is_null() {
        return String::new();
    }
    let len = (0..).take_while(|&i| *text.add(i) != 0).count();
    String::from_utf16_lossy(std::slice::from_raw_parts(text, len))
}
