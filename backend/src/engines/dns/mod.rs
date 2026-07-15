use std::time::Instant;

use hickory_resolver::config::ResolverConfig;
use hickory_resolver::config::ResolverOpts;
use hickory_resolver::TokioAsyncResolver;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// DNS 解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsResult {
    pub dns_time_ms: f64,
    pub dns_success: bool,
    pub resolved_ips: Vec<String>,
}

/// DNS 引擎 — 基于 Hickory DNS
pub struct DnsEngine;

impl DnsEngine {
    /// 解析域名，返回解析耗时和结果
    pub async fn resolve(domain: &str) -> anyhow::Result<DnsResult> {
        let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());

        // 去除协议和路径，只保留域名
        let host = extract_host(domain);
        debug!("DNS 解析: {}", host);

        let start = Instant::now();
        let response = resolver.lookup_ip(host).await;
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;

        match response {
            Ok(lookup) => {
                let ips: Vec<String> = lookup.iter().map(|ip| ip.to_string()).collect();
                let success = !ips.is_empty();
                debug!("DNS 解析成功: {} -> {:?} ({:.2}ms)", host, ips, elapsed);
                Ok(DnsResult {
                    dns_time_ms: elapsed,
                    dns_success: success,
                    resolved_ips: ips,
                })
            }
            Err(e) => {
                debug!("DNS 解析失败: {} - {}", host, e);
                Ok(DnsResult {
                    dns_time_ms: elapsed,
                    dns_success: false,
                    resolved_ips: vec![],
                })
            }
        }
    }
}

/// 从 URL 中提取主机名
fn extract_host(input: &str) -> &str {
    let input = input.trim();
    // 去除协议
    let after_protocol = input
        .split("://")
        .nth(1)
        .unwrap_or(input);
    // 去除路径
    let after_path = after_protocol.split('/').next().unwrap_or(after_protocol);
    // 去除端口
    after_path.split(':').next().unwrap_or(after_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_host() {
        assert_eq!(extract_host("https://www.example.com/path"), "www.example.com");
        assert_eq!(extract_host("http://example.com:8080"), "example.com");
        assert_eq!(extract_host("example.com"), "example.com");
    }
}
