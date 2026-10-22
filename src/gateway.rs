//! Default-route discovery via `/proc/net/route` (Linux fallback).
//!
//! Returns `None` on non-Linux platforms or any parse failure. Callers can
//! instead supply the probe address explicitly via `--probe-ip` / the config
//! `probe_ip` field, which is how captive-redirect discovery works on
//! platforms without /proc (macOS, Windows).
//!
//! We deliberately don't pull in a cross-platform crate (`netdev` etc.):
//! reading the route automatically matters mainly on headless routers where
//! DNS doesn't work pre-auth, and the explicit override lets non-Linux hosts
//! reuse the discovery flow without adding another dependency to the router
//! binary.
//!
//! `/proc/net/route` columns (whitespace-separated):
//!   Iface  Destination  Gateway  Flags  RefCnt  Use  Metric  Mask  MTU  Window  IRTT
//!
//! Gateway is an 8-hex-char IPv4 address in host byte order (little-endian on
//! the platforms we care about), so byte 0 of the hex is octet 4 of the IP.

use std::fs;

/// Return the IPv4 default-gateway address (e.g. `"101.6.40.1"`), or `None` if
/// it can't be determined. If multiple default routes exist, returns the one
/// with the lowest metric (the kernel's preferred path).
pub fn default_gateway_v4() -> Option<String> {
    parse_proc_route(&fs::read_to_string("/proc/net/route").ok()?)
}

fn parse_proc_route(text: &str) -> Option<String> {
    let mut best: Option<(u32, String)> = None;
    for line in text.lines().skip(1) {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 8 || f[1] != "00000000" {
            continue;
        }
        let metric: u32 = f[6].parse().unwrap_or(u32::MAX);
        let Some(ip) = decode_hex_ip_le(f[2]) else {
            continue;
        };
        let is_better = best.as_ref().map_or(true, |(m, _)| metric < *m);
        if is_better {
            best = Some((metric, ip));
        }
    }
    best.map(|(_, ip)| ip)
}

fn decode_hex_ip_le(hex: &str) -> Option<String> {
    if hex.len() != 8 {
        return None;
    }
    let mut b = [0u8; 4];
    for (i, item) in b.iter_mut().enumerate() {
        *item = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    if b == [0; 4] {
        return None; // 0.0.0.0 isn't a useful gateway
    }
    Some(format!("{}.{}.{}.{}", b[3], b[2], b[1], b[0]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real output we observed from a Tsinghua-network OpenWrt 24.10 box;
    /// `ip route show default` reported `default via 101.6.40.1 dev eth0`.
    #[test]
    fn decodes_real_openwrt_default() {
        let text = "\
Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
eth0\t00000000\t01280665\t0003\t0\t0\t0\t00000000\t0\t0\t0
eth0\t00280665\t00000000\t0001\t0\t0\t0\t00FCFFFF\t0\t0\t0
br-lan\t00CAA8C0\t00000000\t0001\t0\t0\t0\t00FFFFFF\t0\t0\t0";
        assert_eq!(parse_proc_route(text), Some("101.6.40.1".into()));
    }

    #[test]
    fn picks_lowest_metric_when_multiple_defaults() {
        let text = "\
Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
eth0\t00000000\t0101A8C0\t0003\t0\t0\t100\t00000000\t0\t0\t0
wlan0\t00000000\t0A0A0A0A\t0003\t0\t0\t10\t00000000\t0\t0\t0";
        // 192.168.1.1 has metric 100; 10.10.10.10 has metric 10 → wlan0 wins.
        assert_eq!(parse_proc_route(text), Some("10.10.10.10".into()));
    }

    #[test]
    fn returns_none_when_no_default_route() {
        let text = "\
Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
eth0\t00280665\t00000000\t0001\t0\t0\t0\t00FCFFFF\t0\t0\t0";
        assert_eq!(parse_proc_route(text), None);
    }

    #[test]
    fn rejects_zero_gateway() {
        // A 0.0.0.0 gateway is meaningless; skip it.
        let text = "\
Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
eth0\t00000000\t00000000\t0003\t0\t0\t0\t00000000\t0\t0\t0";
        assert_eq!(parse_proc_route(text), None);
    }

    #[test]
    fn decodes_ip_byte_order() {
        // 0x01280665 → bytes [0x01, 0x28, 0x06, 0x65] → octets reversed → 101.6.40.1
        assert_eq!(decode_hex_ip_le("01280665"), Some("101.6.40.1".into()));
        // 0xC0A80101 → bytes [0xC0, 0xA8, 0x01, 0x01] → 1.1.168.192
        assert_eq!(decode_hex_ip_le("0101A8C0"), Some("192.168.1.1".into()));
    }
}
