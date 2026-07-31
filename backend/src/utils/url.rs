use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use tokio::net::lookup_host;

/// 验证 URL 是否安全（防 SSRF）
pub fn validate_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("URL 不能为空".into());
    }
    if url.len() > 2048 {
        return Err("URL 过长".into());
    }

    let parsed = url::Url::parse(url).map_err(|_| format!("无效的 URL: {}", url))?;

    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(format!("不支持的协议: {}（仅支持 http/https）", scheme));
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| "无法解析主机名".to_string())?;

    // 解析 IP 地址
    if let Ok(ip) = host.trim_matches(['[', ']']).parse::<IpAddr>() {
        if is_private_ip(&ip) {
            return Err(format!("不允许访问内网地址: {}", host));
        }
    }

    Ok(())
}

/// Validate a URL and resolve its host, rejecting private addresses returned by DNS.
pub async fn validate_url_safety(url: &str, timeout: Duration) -> Result<(), String> {
    validate_url(url)?;
    let parsed = url::Url::parse(url).map_err(|e| format!("URL 解析失败: {}", e))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| "无法解析主机名".to_string())?;
    if host.parse::<IpAddr>().is_ok() {
        return Ok(());
    }
    let port = parsed.port_or_known_default().unwrap_or(80);
    let addresses = tokio::time::timeout(timeout, lookup_host((host, port)))
        .await
        .map_err(|_| "DNS 解析超时".to_string())?
        .map_err(|e| format!("DNS 解析失败: {}", e))?;
    for address in addresses {
        if is_private_ip(&address.ip()) {
            return Err(format!("不允许访问内网地址: {}", host));
        }
    }
    Ok(())
}

pub fn validate_ping_target(target: &str) -> Result<(), String> {
    let target = target.trim();
    if target.is_empty()
        || target.len() > 253
        || target.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Err("无效的 Ping 目标".into());
    }
    let bracketed = target.strip_prefix('[').and_then(|s| s.split_once(']'));
    let host = bracketed
        .map(|(host, _)| host)
        .or_else(|| target.split("//").nth(1).and_then(|s| s.split('/').next()))
        .unwrap_or_else(|| target.split(':').next().unwrap_or(target));
    if host.is_empty()
        || (!bracketed.is_some()
            && host
                .chars()
                .any(|c| !(c.is_alphanumeric() || c == '.' || c == '-' || c == '_')))
        || (bracketed.is_some() && host.parse::<IpAddr>().is_err())
    {
        return Err("无效的 Ping 主机名".into());
    }
    Ok(())
}

/// 检查是否为内网/保留 IP
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_private_ipv4(v4),
        IpAddr::V6(v6) => is_private_ipv6(v6),
    }
}

fn is_private_ipv4(ip: &Ipv4Addr) -> bool {
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.octets()[0] == 0
        // 169.254.x.x (link-local)
        || (ip.octets()[0] == 169 && ip.octets()[1] == 254)
        // 100.x.x.x (CGNAT / 电信级 NAT)
        || (ip.octets()[0] == 100 && (ip.octets()[1] & 0b11000000) == 0b01000000)
        // 198.18.x.x - 198.19.x.x (benchmarking)
        || (ip.octets()[0] == 198 && (ip.octets()[1] == 18 || ip.octets()[1] == 19))
}

fn is_private_ipv6(ip: &Ipv6Addr) -> bool {
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_unique_local()
        || ip.is_multicast()
        // 链路本地地址 (fe80::/10)
        || (ip.segments()[0] & 0xffc0) == 0xfe80
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_urls() {
        assert!(validate_url("https://www.baidu.com").is_ok());
        assert!(validate_url("http://github.com").is_ok());
        assert!(validate_url("https://1.1.1.1").is_ok());
        assert!(validate_url("https://8.8.8.8").is_ok());
    }

    #[test]
    fn test_private_urls() {
        assert!(validate_url("http://127.0.0.1").is_err());
        assert!(validate_url("http://192.168.1.1").is_err());
        assert!(validate_url("http://10.0.0.1").is_err());
        assert!(validate_url("http://172.16.0.1").is_err());
        assert!(validate_url("http://169.254.169.254").is_err());
        assert!(validate_url("http://[::1]").is_err());
    }

    #[test]
    fn test_invalid_scheme() {
        assert!(validate_url("file:///etc/passwd").is_err());
        assert!(validate_url("javascript:alert(1)").is_err());
        assert!(validate_url("ftp://example.com").is_err());
    }

    #[test]
    fn test_cgnat() {
        assert!(validate_url("http://100.64.0.1").is_err());
        assert!(validate_url("http://100.127.255.255").is_err());
    }

    #[test]
    fn test_ping_targets() {
        assert!(validate_ping_target("1.1.1.1:443").is_ok());
        assert!(validate_ping_target("example.com").is_ok());
        assert!(validate_ping_target("[2001:db8::1]").is_ok());
        assert!(validate_ping_target("bad target").is_err());
    }

    #[test]
    fn test_ipv6_url_parsing() {
        assert!(validate_url("https://[2001:4860:4860::8888]/").is_ok());
        assert!(validate_url("https://[fe80::1]/").is_err());
    }
}
