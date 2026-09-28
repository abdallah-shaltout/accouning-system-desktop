//! Terminal pairing (phase-a3 A3-2, P2-57): the Main PC's host name + non-loopback LAN addresses +
//! port + a short human-typeable code (the LAN account's secret, formatted `XXXX-XXXX-XXXX-XXXX`).
//! `parse_code` is the terminal-side inverse — tolerant of case, spacing and missing dashes, and
//! maps the handful of characters that are easy to mis-key against the Crockford base32 alphabet
//! `credentials.rs` generates codes from.

#![cfg(windows)]

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PairingInfo {
    #[serde(rename = "hostName")]
    pub host_name: String,
    pub addresses: Vec<String>,
    pub port: u16,
    pub code: String,
}

/// `GetComputerNameExW(ComputerNameDnsHostname)` — the DNS hostname a terminal can resolve on the
/// LAN even after this PC's IP changes via DHCP (A3-2's "terminal connect fallback" story).
pub fn host_name() -> Option<String> {
    use windows::Win32::System::SystemInformation::{ComputerNameDnsHostname, GetComputerNameExW};
    unsafe {
        let mut len: u32 = 0;
        // First call with a null buffer to get the required length (Win32 convention) — expected to
        // "fail" with the required size written into `len`.
        let _ = GetComputerNameExW(ComputerNameDnsHostname, None, &mut len);
        if len == 0 {
            return None;
        }
        let mut buf = vec![0u16; len as usize];
        let ok = GetComputerNameExW(ComputerNameDnsHostname, Some(windows::core::PWSTR(buf.as_mut_ptr())), &mut len).is_ok();
        if !ok {
            return None;
        }
        // `len` is updated to the actual length written, excluding the null terminator.
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

/// 169.254.0.0/16 — `Ipv4Addr::is_link_local` is nightly-only, so this is checked by hand.
fn is_link_local_v4(ip: std::net::Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 169 && o[1] == 254
}

/// Every non-loopback, non-link-local IPv4 address of an "up" network interface — the addresses a
/// terminal on the same LAN could actually reach this PC on. Uses `GetAdaptersAddresses`
/// (`Win32_NetworkManagement_IpHelper`, plus `Win32_NetworkManagement_Ndis` for
/// `IP_ADAPTER_ADDRESSES_LH`'s `OperStatus` field — **needs adding to `Cargo.toml`**, see the
/// phase status note) rather than parsing `ipconfig` output.
pub fn non_loopback_ipv4_addresses() -> Vec<String> {
    use std::net::Ipv4Addr;

    use windows::Win32::Foundation::ERROR_BUFFER_OVERFLOW;
    use windows::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST,
    };
    use windows::Win32::Networking::WinSock::{AF_INET, SOCKADDR_IN};

    let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
    let mut size: u32 = 15 * 1024;
    let mut buf: Vec<u8>;

    let mut result;
    loop {
        buf = vec![0u8; size as usize];
        let head = buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
        result = unsafe { GetAdaptersAddresses(AF_INET.0 as u32, flags, None, Some(head), &mut size) };
        if result != ERROR_BUFFER_OVERFLOW.0 {
            break;
        }
        // `size` was updated to the required size; loop again with a bigger buffer.
    }
    if result != 0 {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut cursor = buf.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
    unsafe {
        while !cursor.is_null() {
            let adapter = &*cursor;
            // Only "up" interfaces (IfOperStatusUp == 1).
            if adapter.OperStatus.0 == 1 {
                let mut unicast = adapter.FirstUnicastAddress;
                while !unicast.is_null() {
                    let addr = &*unicast;
                    let sockaddr = addr.Address.lpSockaddr;
                    if !sockaddr.is_null() && (*sockaddr).sa_family == AF_INET {
                        let sockaddr_in = sockaddr as *const SOCKADDR_IN;
                        let raw = (*sockaddr_in).sin_addr.S_un.S_addr;
                        let ip = Ipv4Addr::from(u32::from_be(raw));
                        if !ip.is_loopback() && !is_link_local_v4(ip) {
                            out.push(ip.to_string());
                        }
                    }
                    unicast = addr.Next;
                }
            }
            cursor = adapter.Next;
        }
    }
    out
}

/// Formats a raw secret (any length, Crockford base32 alphabet) as groups of 4 separated by
/// dashes: `XXXX-XXXX-XXXX-XXXX` for a 16-char secret. Any leftover characters that don't fill a
/// full group of 4 are appended as a final short group (kept simple: `enable_lan_sharing` always
/// generates exactly 16-char secrets, so in production this is always exactly 4 groups of 4).
pub fn format_code(secret: &str) -> String {
    secret.as_bytes().chunks(4).map(|c| std::str::from_utf8(c).unwrap_or("")).collect::<Vec<_>>().join("-")
}

/// Crockford base32 alphabet, same as `credentials.rs::generate_secret` — kept in sync manually
/// since duplicating the `const` here (rather than making it `pub` and importing it) keeps this
/// module fully independent of the codegen path a terminal that never provisions anything doesn't
/// need to pull in.
const ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Parses a pairing code typed by a cashier on a terminal: tolerant of lowercase, extra spaces and
/// missing dashes, and maps the handful of Crockford "confusable" characters (`O`→`0`, `I`/`L`→`1`)
/// before validating length and alphabet. Returns the normalized (uppercase, unpunctuated) secret,
/// or `None` if it can't possibly be a valid code.
pub fn parse_code(input: &str) -> Option<String> {
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .map(|c| match c {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        })
        .collect();

    if cleaned.len() != 16 {
        return None;
    }
    if !cleaned.chars().all(|c| ALPHABET.contains(c)) {
        return None;
    }
    Some(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_code_groups_a_16_char_secret_into_four_dashed_quads() {
        let code = format_code("ABCD1234EFGH5678");
        assert_eq!(code, "ABCD-1234-EFGH-5678");
    }

    #[test]
    fn parse_code_accepts_dashes_case_and_spaces() {
        let raw = "ABCD-1234-EFGH-5678";
        assert_eq!(parse_code(raw), Some("ABCD1234EFGH5678".to_string()));
        assert_eq!(parse_code("abcd 1234 efgh 5678"), Some("ABCD1234EFGH5678".to_string()));
        assert_eq!(parse_code("abcd1234efgh5678"), Some("ABCD1234EFGH5678".to_string()));
    }

    #[test]
    fn parse_code_maps_crockford_confusables() {
        // O -> 0, I/L -> 1
        assert_eq!(parse_code("OOOO-IIII-LLLL-0000"), Some("0000111111110000".to_string()));
    }

    #[test]
    fn parse_code_rejects_bad_length() {
        assert_eq!(parse_code("ABCD-1234"), None);
        assert_eq!(parse_code("ABCD-1234-EFGH-56789"), None);
    }

    #[test]
    fn parse_code_rejects_characters_outside_the_alphabet_after_normalization() {
        // 'U' is not in the Crockford alphabet and isn't one of the mapped confusables.
        assert_eq!(parse_code("UUUU-1234-EFGH-5678"), None);
    }
}
