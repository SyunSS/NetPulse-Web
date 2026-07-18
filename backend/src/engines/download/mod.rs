use std::net::ToSocketAddrs;
use std::time::{Duration, Instant};

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tracing::{debug, info, warn};
use url::Url;

/// 下载测试结果（含 DNS/TCP 探测）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTestResult {
    pub download_speed: f64,
    pub avg_speed: f64,
    pub peak_speed: f64,
    pub download_time_ms: f64,
    pub file_size: i32,
    pub dns_time_ms: Option<f64>,
    pub dns_success: Option<i32>,
    pub tcp_time_ms: Option<f64>,
    pub success: bool,
    pub error: Option<String>,
}

/// 下载引擎 — 基于 reqwest streaming + 独立 DNS/TCP 探测
pub struct DownloadEngine {
    timeout: Duration,
    max_duration: Duration,
}

impl DownloadEngine {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout, max_duration: Duration::from_secs(15) }
    }

    /// 执行下载测试（含 DNS + TCP 预探测）
    pub async fn test_download(&self, url_str: &str) -> DownloadTestResult {
        info!("下载测试开始: {}", url_str);

        // 1. DNS + TCP 预探测
        let (dns_time, dns_success, tcp_time) = probe_dns_tcp(url_str).await;

        // 2. HTTP 下载
        let client = match reqwest::Client::builder()
            .timeout(self.timeout)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
        {
            Ok(c) => c,
            Err(e) => return download_err("创建客户端失败", &e.to_string(), dns_time, dns_success, tcp_time),
        };

        let response = match client.get(url_str).send().await {
            Ok(r) => r,
            Err(e) => return download_err("请求失败", &e.to_string(), dns_time, dns_success, tcp_time),
        };

        if !response.status().is_success() {
            return download_err("HTTP错误", &format!("{}", response.status().as_u16()),
                dns_time, dns_success, tcp_time);
        }

        // 流式下载 + 分段计速
        let mut total_bytes = 0u64; let mut peak_speed = 0.0f64;
        let mut speed_samples: Vec<f64> = Vec::new();
        let mut segment_bytes = 0u64; let mut last_report = Instant::now();
        let download_start = Instant::now();
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = match chunk { Ok(c) => c, Err(e) => { warn!("流错误: {}", e); break; } };
            let len = chunk.len() as u64; total_bytes += len; segment_bytes += len;
            let elapsed = last_report.elapsed();
            if elapsed >= Duration::from_secs(1) {
                let speed = segment_bytes as f64 / elapsed.as_secs_f64() / 1024.0;
                speed_samples.push(speed);
                if speed > peak_speed { peak_speed = speed; }
                segment_bytes = 0; last_report = Instant::now();
            }
            if download_start.elapsed() >= self.max_duration { break; }
        }
        if segment_bytes > 0 {
            let elapsed = last_report.elapsed().as_secs_f64();
            if elapsed > 0.01 {
                let speed = segment_bytes as f64 / elapsed / 1024.0;
                speed_samples.push(speed); if speed > peak_speed { peak_speed = speed; }
            }
        }

        let elapsed_ms = download_start.elapsed().as_secs_f64() * 1000.0;
        let avg_speed = if elapsed_ms > 0.0 && total_bytes > 0 {
            total_bytes as f64 / (elapsed_ms / 1000.0) / 1024.0
        } else { 0.0 };
        let current_speed = speed_samples.last().copied().unwrap_or(avg_speed);

        DownloadTestResult {
            download_speed: current_speed, avg_speed, peak_speed,
            download_time_ms: elapsed_ms, file_size: total_bytes as i32,
            dns_time_ms: Some(dns_time), dns_success: Some(dns_success),
            tcp_time_ms: Some(tcp_time),
            success: true, error: None,
        }
    }
}

/// DNS + TCP 预探测（独立于 reqwest）
async fn probe_dns_tcp(url_str: &str) -> (f64, i32, f64) {
    let url = match Url::parse(url_str) {
        Ok(u) => u,
        Err(_) => return (0.0, 0, 0.0),
    };
    let host = url.host_str().unwrap_or("");
    let port = url.port().unwrap_or(if url.scheme() == "https" { 443 } else { 80 });

    // DNS 解析
    let dns_start = Instant::now();
    let addr_str = format!("{}:{}", host, port);
    let dns_result = tokio::task::spawn_blocking(move || addr_str.to_socket_addrs()
        .ok().and_then(|mut a| a.next())
    ).await.ok().flatten();
    let dns_time = dns_start.elapsed().as_secs_f64() * 1000.0;
    let dns_success = if dns_result.is_some() { 100 } else { 0 };

    // TCP 连接
    let tcp_time = if let Some(addr) = dns_result {
        let start = Instant::now();
        match tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(addr)).await {
            Ok(Ok(_)) => start.elapsed().as_secs_f64() * 1000.0,
            _ => 0.0,
        }
    } else { 0.0 };

    (dns_time, dns_success, tcp_time)
}

fn download_err(kind: &str, msg: &str, dns_time: f64, dns_success: i32, tcp_time: f64) -> DownloadTestResult {
    DownloadTestResult {
        download_speed: 0.0, avg_speed: 0.0, peak_speed: 0.0,
        download_time_ms: 0.0, file_size: 0,
        dns_time_ms: Some(dns_time), dns_success: Some(dns_success), tcp_time_ms: Some(tcp_time),
        success: false, error: Some(format!("{}: {}", kind, msg)),
    }
}
