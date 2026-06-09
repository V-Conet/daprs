//! 输入验证模块
//!
//! 提供统一的输入验证函数，供 Agent 和 Server 共用。

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// ASN 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsnType {
    /// DN42: 4242420000-4242429999
    Dn42,
    /// NeoNetwork: 4201270000-4201280000
    NeoNetwork,
    /// DN42 Legacy: 64512-65534 或 4200000000-4295000000
    Dn42Legacy,
    /// 公共 ASN
    Public,
}

impl AsnType {
    /// 根据 ASN 号判断类型
    pub fn from_asn(asn: u32) -> Self {
        if (4242420000..=4242429999).contains(&asn) {
            AsnType::Dn42
        } else if (4201270000..=4201280000).contains(&asn) {
            AsnType::NeoNetwork
        } else if (64512..=65534).contains(&asn) || (4200000000..=4294967295).contains(&asn) {
            AsnType::Dn42Legacy
        } else {
            AsnType::Public
        }
    }

    /// 是否为私有 ASN（DN42/NeoNetwork/Legacy）
    pub fn is_private(&self) -> bool {
        matches!(
            self,
            AsnType::Dn42 | AsnType::NeoNetwork | AsnType::Dn42Legacy
        )
    }

    /// 根据 ASN 计算默认端口
    /// DN42: 后5位
    /// NeoNetwork/Dn42Legacy: 30000 + 后4位
    /// Public: 40000 + 后4位
    pub fn default_port(&self, asn: u32) -> u16 {
        let last4 = (asn % 10000) as u16;
        match self {
            AsnType::Dn42 => (asn % 100000) as u16,
            AsnType::NeoNetwork | AsnType::Dn42Legacy => 30000 + last4,
            AsnType::Public => 40000 + last4,
        }
    }
}

/// 验证 ASN 是否合法
///
/// 仅拒绝 0（无效 ASN）
pub fn validate_asn(asn: u32) -> Result<(), &'static str> {
    if asn == 0 {
        return Err("ASN cannot be zero");
    }
    Ok(())
}

/// 验证 DN42 IPv4 地址
///
/// DN42: 172.20.0.0/14
/// NeoNetwork: 10.127.0.0/16
pub fn validate_dn42_ipv4(addr: &str) -> Result<Ipv4Addr, &'static str> {
    let ip: Ipv4Addr = addr.parse().map_err(|_| "Invalid IPv4 format")?;

    let dn42_network = ipnet::Ipv4Net::new(Ipv4Addr::new(172, 20, 0, 0), 14).unwrap();
    let neo_network = ipnet::Ipv4Net::new(Ipv4Addr::new(10, 127, 0, 0), 16).unwrap();

    if dn42_network.contains(&ip) || neo_network.contains(&ip) {
        Ok(ip)
    } else {
        Err("IPv4 must be in DN42 (172.20.0.0/14) or NeoNetwork (10.127.0.0/16) range")
    }
}

/// 验证 DN42 IPv6 地址
///
/// ULA: fc00::/7
/// Link-Local: fe80::/10 (实际使用 fe80::/64)
pub fn validate_dn42_ipv6(addr: &str) -> Result<Ipv6Addr, &'static str> {
    let ip: Ipv6Addr = addr.parse().map_err(|_| "Invalid IPv6 format")?;

    // ULA: fc00::/7 (fc00::/8 或 fd00::/8)
    let is_ula = (ip.segments()[0] & 0xfe00) == 0xfc00;
    // Link-Local: fe80::/10
    let is_link_local = (ip.segments()[0] & 0xffc0) == 0xfe80;

    if is_ula || is_link_local {
        Ok(ip)
    } else {
        Err("IPv6 must be ULA (fc00::/7) or Link-Local (fe80::/10)")
    }
}

/// 验证 WireGuard 公钥/预共享密钥
///
/// 必须为 44 字符的 base64 编码字符串，以 '=' 结尾
pub fn validate_wg_key(key: &str) -> Result<(), &'static str> {
    if key.len() != 44 {
        return Err("WireGuard key must be 44 characters");
    }
    if !key.ends_with('=') {
        return Err("WireGuard key must end with '='");
    }
    // 验证 base64 字符
    if !key
        .chars()
        .take(43)
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/')
    {
        return Err("Invalid base64 characters in WireGuard key");
    }
    Ok(())
}

/// 验证 Endpoint 格式
///
/// 格式: host:port 或 [ipv6]:port
/// 拒绝特殊字符（命令注入防护）
pub fn validate_endpoint(endpoint: &str) -> Result<(&str, u16), &'static str> {
    if endpoint.is_empty() || endpoint.len() > 256 {
        return Err("Endpoint must be 1-256 characters");
    }

    // 禁止特殊字符
    if endpoint.contains(['\n', '\r', '\0', '"', '\'']) {
        return Err("Invalid characters in endpoint");
    }

    // 解析 host:port 或 [ipv6]:port
    let (host, port_str) = if endpoint.starts_with('[') {
        // IPv6 格式: [addr]:port
        let close = endpoint.find(']').ok_or("Invalid IPv6 endpoint format")?;
        let host = &endpoint[1..close];
        let port_str = endpoint
            .get(close + 2..)
            .ok_or("Missing port in endpoint")?;
        (host, port_str)
    } else {
        // host:port 格式
        let colon = endpoint.rfind(':').ok_or("Missing port in endpoint")?;
        (&endpoint[..colon], &endpoint[colon + 1..])
    };

    // 验证端口
    let port: u16 = port_str.parse().map_err(|_| "Invalid port number")?;
    if port == 0 {
        return Err("Port cannot be zero");
    }

    Ok((host, port))
}

/// 验证 MTU 范围
pub fn validate_mtu(mtu: u16) -> Result<(), &'static str> {
    if !(576..=9000).contains(&mtu) {
        return Err("MTU must be between 576 and 9000");
    }
    Ok(())
}

/// 验证端口范围
pub fn validate_port(port: u16) -> Result<(), &'static str> {
    if port < 1024 {
        return Err("Port must be >= 1024");
    }
    Ok(())
}

/// 检查 IP 是否为 bogon 地址
///
/// 参考: https://en.wikipedia.org/wiki/Bogon_filtering
pub fn is_bogon_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_bogon_ipv4(v4),
        IpAddr::V6(v6) => is_bogon_ipv6(v6),
    }
}

/// IPv4 Bogon 范围
fn is_bogon_ipv4(ip: &Ipv4Addr) -> bool {
    let octets = ip.octets();

    // 0.0.0.0/8
    if octets[0] == 0 {
        return true;
    }
    // 10.0.0.0/8 (private, but DN42/NeoNetwork use this)
    // 100.64.0.0/10 (CGN)
    if octets[0] == 100 && (64..=127).contains(&octets[1]) {
        return true;
    }
    // 127.0.0.0/8 (loopback)
    if octets[0] == 127 {
        return true;
    }
    // 169.254.0.0/16 (link-local)
    if octets[0] == 169 && octets[1] == 254 {
        return true;
    }
    // 172.16.0.0/12 (private, but DN42 uses 172.20.0.0/14)
    // 192.0.0.0/24
    if octets[0] == 192 && octets[1] == 0 && octets[2] == 0 {
        return true;
    }
    // 192.0.2.0/24 (documentation)
    if octets[0] == 192 && octets[1] == 0 && octets[2] == 2 {
        return true;
    }
    // 192.168.0.0/16 (private)
    // 198.18.0.0/15 (benchmarking)
    if octets[0] == 198 && (18..=19).contains(&octets[1]) {
        return true;
    }
    // 198.51.100.0/24 (documentation)
    if octets[0] == 198 && octets[1] == 51 && octets[2] == 100 {
        return true;
    }
    // 203.0.113.0/24 (documentation)
    if octets[0] == 203 && octets[1] == 0 && octets[2] == 113 {
        return true;
    }
    // 224.0.0.0/4 (multicast)
    if octets[0] >= 224 && octets[0] <= 239 {
        return true;
    }
    // 240.0.0.0/4 (reserved)
    if octets[0] >= 240 {
        return true;
    }

    false
}

/// IPv6 Bogon 范围
fn is_bogon_ipv6(ip: &Ipv6Addr) -> bool {
    let segments = ip.segments();

    // ::/128 (unspecified)
    if ip.is_unspecified() {
        return true;
    }
    // ::1/128 (loopback)
    if ip.is_loopback() {
        return true;
    }
    // ::ffff:0:0/96 (IPv4-mapped)
    if segments[0..5] == [0, 0, 0, 0, 0] && segments[5] == 0xffff {
        return true;
    }
    // 100::/64 (discard-only)
    if segments[0] == 0x100 && segments[1..8] == [0, 0, 0, 0, 0, 0, 0] {
        return true;
    }
    // 2001:10::/28 (ORCHID)
    if segments[0] == 0x2001 && (segments[1] >> 4) == 0x10 >> 4 {
        return true;
    }
    // 2001:db8::/32 (documentation)
    if segments[0] == 0x2001 && segments[1] == 0xdb8 {
        return true;
    }
    // fc00::/7 (ULA - DN42 uses this, not bogon for our purposes)
    // fe80::/10 (link-local - DN42 uses this, not bogon for our purposes)

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asn_type() {
        assert_eq!(AsnType::from_asn(4242420000), AsnType::Dn42);
        assert_eq!(AsnType::from_asn(4242421234), AsnType::Dn42);
        assert_eq!(AsnType::from_asn(4201270000), AsnType::NeoNetwork);
        assert_eq!(AsnType::from_asn(64512), AsnType::Dn42Legacy);
        assert_eq!(AsnType::from_asn(1234), AsnType::Public);
    }

    #[test]
    fn test_asn_port() {
        assert_eq!(AsnType::default_port(&AsnType::Dn42, 4242421234), 21234);
        assert_eq!(
            AsnType::default_port(&AsnType::NeoNetwork, 4201270001),
            30001
        );
        assert_eq!(AsnType::default_port(&AsnType::Public, 1234), 41234);
    }

    #[test]
    fn test_validate_dn42_ipv4() {
        assert!(validate_dn42_ipv4("172.20.0.1").is_ok());
        assert!(validate_dn42_ipv4("10.127.0.1").is_ok());
        assert!(validate_dn42_ipv4("192.168.1.1").is_err());
        assert!(validate_dn42_ipv4("8.8.8.8").is_err());
    }

    #[test]
    fn test_validate_dn42_ipv6() {
        assert!(validate_dn42_ipv6("fd00::1").is_ok());
        assert!(validate_dn42_ipv6("fc00::1").is_ok());
        assert!(validate_dn42_ipv6("fe80::1").is_ok());
        assert!(validate_dn42_ipv6("2001:db8::1").is_err());
    }

    #[test]
    fn test_validate_wg_key() {
        let valid_key = "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXoxMjM0NTY=";
        assert!(validate_wg_key(valid_key).is_ok());
        assert!(validate_wg_key("short").is_err());
        assert!(validate_wg_key("YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXoxMjM0NTY").is_err());
    }

    #[test]
    fn test_validate_endpoint() {
        assert!(validate_endpoint("example.com:51820").is_ok());
        assert!(validate_endpoint("[2001:db8::1]:51820").is_ok());
        assert!(validate_endpoint("192.168.1.1:51820").is_ok());
        assert!(validate_endpoint("example.com").is_err());
        assert!(validate_endpoint("example.com:0").is_err());
    }
}
