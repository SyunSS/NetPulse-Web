use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tracing::{debug, info};

/// Ping 测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingTestResult {
    pub host: String,
    pub avg_latency_ms: f64,
    pub packet_loss_rate: f64,  // 0-100
    pub jitter_ms: f64,
    pub success: bool,
    pub method: Option<String>,  // "icmp" | "tcp80" | "tcp443"
    pub error: Option<String>,
}

/// Ping 引擎 — 使用系统 ping 命令
pub struct PingEngine {
    count: u32,
    timeout: Duration,
}

impl PingEngine {
    pub fn new(count: u32, timeout: Duration) -> Self {
        Self { count: count.max(1).min(100), timeout }
    }

    /// 执行 ping 测试（ICMP → TCP:80 → TCP:443 三级回退）
    pub async fn test_ping(&self, host: &str) -> PingTestResult {
        info!("Ping 测试开始: {}", host);
        let target = extract_host(host);

        // 1. ICMP
        let count = self.count;
        let timeout_secs = self.timeout.as_secs();
        let icmp_target = target.clone();
        let icmp_result = tokio::task::spawn_blocking(move || {
            run_ping(&icmp_target, count, timeout_secs)
        }).await;

        match icmp_result {
            Ok(Ok(mut r)) if r.success => { r.method = Some("icmp".into()); return r; }
            Ok(Ok(r)) => info!("ICMP 不通 {} (丢包{}%, rtts=0), 回退 TCP", r.host, r.packet_loss_rate),
            Ok(Err(e)) => info!("ICMP 失败 {}, 回退 TCP", e),
            Err(e) => info!("ICMP 异常 {}, 回退 TCP", e),
        }

        // 2. TCP :80
        info!("TCP 回退: {}:80", target);
        let tcp80 = run_tcp_ping(&target, 80, 5).await;
        if tcp80.success {
            return tcp80;
        }
        debug!("TCP:80 不通, 尝试 TCP:443");

        // 3. TCP :443
        info!("TCP 回退: {}:443", target);
        let tcp443 = run_tcp_ping(&target, 443, 5).await;
        if tcp443.success {
            return tcp443;
        }

        // 全部失败
        PingTestResult {
            host: target,
            avg_latency_ms: 0.0,
            packet_loss_rate: 100.0,
            jitter_ms: 0.0,
            success: false,
            method: None,
            error: Some("ICMP/TCP80/TCP443 全部不通".into()),
        }
    }
}

/// 执行系统 ping 命令并解析结果
fn run_ping(host: &str, count: u32, timeout_secs: u64) -> Result<PingTestResult, String> {
    let output = std::process::Command::new("ping")
        .arg("-c")
        .arg(count.to_string())
        .arg("-W")
        .arg((timeout_secs.max(1)).to_string())
        .arg("-i")
        .arg("0.5")  // 间隔 0.5 秒
        .arg(host)
        .output()
        .map_err(|e| format!("执行 ping 失败: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    debug!("ping 输出:\n{}", stdout);

    if !output.status.success() && stdout.is_empty() {
        return Err(format!("ping 失败: {}", stderr.trim()));
    }

    // 解析 RTT 值（从 icmp_seq=... time=XX.X ms 行）
    let mut rtts: Vec<f64> = Vec::new();
    for line in stdout.lines() {
        if line.contains("time=") {
            if let Some(time_part) = line.split("time=").nth(1) {
                if let Some(num_str) = time_part.split_whitespace().next() {
                    if let Ok(ms) = num_str.parse::<f64>() {
                        rtts.push(ms);
                    }
                }
            }
        }
    }

    // 解析丢包率
    let mut packet_loss = 100.0;
    for line in stdout.lines() {
        if line.contains("packet loss") || line.contains("丢包") {
            // 格式: "0% packet loss" 或 "0% 丢包率"
            if let Some(pct_str) = line.split('%').next() {
                if let Some(num) = pct_str.split_whitespace().last() {
                    if let Ok(p) = num.parse::<f64>() {
                        packet_loss = p;
                        break;
                    }
                }
            }
        }
    }

    // 解析平均延迟（从 rtt min/avg/max/mdev 行）
    let mut avg_latency = 0.0;
    let mut jitter = 0.0;
    for line in stdout.lines() {
        if line.contains("rtt") || line.contains("round-trip") {
            // 格式: "rtt min/avg/max/mdev = 1.234/5.678/9.012/3.456 ms"
            if let Some(stats_part) = line.split('=').nth(1) {
                let nums: Vec<f64> = stats_part
                    .split('/')
                    .filter_map(|s| s.trim().split_whitespace().next().and_then(|n| n.parse().ok()))
                    .collect();
                if nums.len() >= 4 {
                    avg_latency = nums[1];  // avg
                    jitter = nums[3];       // mdev (mean deviation)
                }
            }
        }
    }

    // 如果没解析到 avg，用手动计算的值
    if avg_latency == 0.0 && !rtts.is_empty() {
        avg_latency = rtts.iter().sum::<f64>() / rtts.len() as f64;
        // 手动计算抖动（标准差）
        let mean = avg_latency;
        let variance = rtts.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / rtts.len() as f64;
        jitter = variance.sqrt();
    }

    let success = packet_loss < 100.0 && (avg_latency > 0.0 || !rtts.is_empty());

    Ok(PingTestResult {
        host: host.to_string(),
        avg_latency_ms: (avg_latency * 1000.0).round() / 1000.0,
        packet_loss_rate: (packet_loss * 100.0).round() / 100.0,
        jitter_ms: (jitter * 1000.0).round() / 1000.0,
        success,
        method: Some("icmp".into()),
        error: if success { None } else { Some("100% 丢包".to_string()) },
    })
}

/// TCP Ping: 连接指定端口，连 N 次取平均延迟
async fn run_tcp_ping(host: &str, port: u16, count: u32) -> PingTestResult {
    let addr = format!("{}:{}", host, port);
    let method = format!("tcp{}", port);
    let mut times: Vec<f64> = Vec::new();
    let mut fails = 0u32;
    let mut last_err = String::new();

    for _ in 0..count {
        let start = Instant::now();
        match tokio::time::timeout(Duration::from_secs(3), TcpStream::connect(addr.clone())).await {
            Ok(Ok(stream)) => {
                times.push(start.elapsed().as_secs_f64() * 1000.0);
                drop(stream);
            }
            Ok(Err(e)) => { fails += 1; last_err = e.to_string(); }
            Err(_timeout) => { fails += 1; last_err = "timeout".into(); }
        }
    }

    info!("TCP ping {}:{} 结果: {}/{} 成功, 时延={:?}ms, err={}", host, port, times.len(), count,
        if times.is_empty() { None } else { Some(times.iter().sum::<f64>() / times.len() as f64) }, last_err);

    if times.is_empty() {
        return PingTestResult {
            host: host.to_string(), avg_latency_ms: 0.0, packet_loss_rate: 100.0,
            jitter_ms: 0.0, success: false, method: Some(method),
            error: Some(format!("TCP:{} {}, {}次全失败", port, last_err, count)),
        };
    }

    let avg = times.iter().sum::<f64>() / times.len() as f64;
    let loss = (fails as f64 / count as f64) * 100.0;
    let jitter = if times.len() > 1 {
        let variance = times.iter().map(|t| (t - avg).powi(2)).sum::<f64>() / times.len() as f64;
        variance.sqrt()
    } else { 0.0 };

    PingTestResult {
        host: host.to_string(),
        avg_latency_ms: (avg * 1000.0).round() / 1000.0,
        packet_loss_rate: (loss * 100.0).round() / 100.0,
        jitter_ms: (jitter * 1000.0).round() / 1000.0,
        success: loss < 100.0,
        method: Some(method),
        error: if loss >= 100.0 { Some(format!("TCP:{} 全部超时或拒绝", port)) } else { None },
    }
}

/// 从 URL 提取主机名
fn extract_host(input: &str) -> String {
    let input = input.trim();
    let after_protocol = input.split("://").nth(1).unwrap_or(input);
    let after_path = after_protocol.split('/').next().unwrap_or(after_protocol);
    let after_port = after_path.split(':').next().unwrap_or(after_path);
    let host = after_port.to_string();
    // 处理 IPv6: 去掉方括号
    if host.starts_with('[') && host.ends_with(']') {
        host[1..host.len()-1].to_string()
    } else {
        host
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_host() {
        assert_eq!(extract_host("https://www.example.com/path"), "www.example.com");
        assert_eq!(extract_host("http://example.com:8080"), "example.com");
        assert_eq!(extract_host("8.8.8.8"), "8.8.8.8");
        assert_eq!(extract_host("8.8.8.8:53"), "8.8.8.8");
    }
}
