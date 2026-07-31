use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 简单的内存频率限制器
pub struct RateLimiter {
    attempts: Mutex<HashMap<String, Vec<Instant>>>,
    max_attempts: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_attempts: usize, window_secs: u64) -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
            max_attempts,
            window: Duration::from_secs(window_secs),
        }
    }

    /// 检查是否允许请求。返回 `false` 表示超出限制。
    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut map = self.attempts.lock().unwrap();

        let timestamps = map.entry(key.to_string()).or_insert_with(Vec::new);

        // 清理过期记录
        timestamps.retain(|t| now.duration_since(*t) < self.window);

        if timestamps.len() >= self.max_attempts {
            return false;
        }

        timestamps.push(now);
        true
    }
}

pub fn ip_matches(entry: &str, ip: IpAddr) -> bool {
    let entry = entry.trim();
    if let Ok(expected) = entry.parse::<IpAddr>() {
        return expected == ip;
    }
    let Some((network, prefix)) = entry.split_once('/') else {
        return false;
    };
    let Ok(network) = network.parse::<IpAddr>() else {
        return false;
    };
    let Ok(prefix) = prefix.parse::<u8>() else {
        return false;
    };
    match (network, ip) {
        (IpAddr::V4(network), IpAddr::V4(ip)) if prefix <= 32 => {
            let mask = if prefix == 0 {
                0
            } else {
                u32::MAX << (32 - prefix)
            };
            u32::from(network) & mask == u32::from(ip) & mask
        }
        (IpAddr::V6(network), IpAddr::V6(ip)) if prefix <= 128 => {
            let mask = if prefix == 0 {
                0
            } else {
                u128::MAX << (128 - prefix)
            };
            u128::from(network) & mask == u128::from(ip) & mask
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_explicit_proxy_networks_only() {
        assert!(ip_matches("127.0.0.1", "127.0.0.1".parse().unwrap()));
        assert!(ip_matches("10.0.0.0/8", "10.2.3.4".parse().unwrap()));
        assert!(!ip_matches("10.0.0.0/8", "192.168.1.1".parse().unwrap()));
        assert!(!ip_matches("bad", "127.0.0.1".parse().unwrap()));
    }
}
