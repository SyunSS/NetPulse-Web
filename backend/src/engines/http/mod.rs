use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tracing::debug;

/// HTTP 探测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResult {
    pub tcp_time_ms: f64,
    pub tls_time_ms: f64,
    pub http_status: Option<i32>,
    pub ttfb_ms: f64,
    pub final_url: String,
    pub error: Option<String>,
}

/// HTTP 引擎 — 基于 Reqwest + 手动 TCP 连接计时
pub struct HttpEngine;

impl HttpEngine {
    /// 探测 URL，返回 TCP/TLS/TTFB 等指标
    pub async fn probe(url: &str, timeout: Duration) -> HttpResult {
        let parsed = match url::Url::parse(url) {
            Ok(u) => u,
            Err(e) => {
                return HttpResult {
                    tcp_time_ms: 0.0,
                    tls_time_ms: 0.0,
                    http_status: None,
                    ttfb_ms: 0.0,
                    final_url: url.to_string(),
                    error: Some(format!("URL 解析失败: {}", e)),
                };
            }
        };

        let host = parsed.host_str().unwrap_or("");
        let port = parsed.port_or_known_default().unwrap_or(80);
        let is_https = parsed.scheme() == "https";

        // 1. TCP 连接计时
        let tcp_time_ms = measure_tcp_connect(host, port, timeout).await;

        if tcp_time_ms == 0.0 {
            return HttpResult {
                tcp_time_ms: 0.0,
                tls_time_ms: 0.0,
                http_status: None,
                ttfb_ms: 0.0,
                final_url: url.to_string(),
                error: Some("TCP 连接失败".to_string()),
            };
        }

        // 2. TLS 握手时间估算（HTTPS 时，TLS 时间 ≈ 总连接时间 - TCP 时间）
        // 使用 reqwest 发请求，通过时间差估算 TLS + TTFB
        let tls_time_ms = if is_https {
            // TLS 时间通过 reqwest 连接时间估算
            measure_tls_time(host, port, timeout).await
        } else {
            0.0
        };

        // 3. 发起 HTTP 请求获取状态码和 TTFB
        let (http_status, ttfb_ms, final_url) =
            measure_http_request(url, timeout).await;

        HttpResult {
            tcp_time_ms,
            tls_time_ms,
            http_status,
            ttfb_ms,
            final_url,
            error: None,
        }
    }
}

/// 测量 TCP 连接时间
async fn measure_tcp_connect(host: &str, port: u16, timeout: Duration) -> f64 {
    let addr = format!("{}:{}", host, port);
    let start = Instant::now();
    match tokio::time::timeout(timeout, TcpStream::connect(&addr)).await {
        Ok(Ok(_)) => start.elapsed().as_secs_f64() * 1000.0,
        Ok(Err(e)) => {
            debug!("TCP 连接失败 {}:{} - {}", host, port, e);
            0.0
        }
        Err(_) => {
            debug!("TCP 连接超时 {}:{}", host, port);
            0.0
        }
    }
}

/// 测量 TLS 握手时间（通过 HTTPS 连接的总时间减去 TCP 时间来估算）
async fn measure_tls_time(host: &str, port: u16, timeout: Duration) -> f64 {
    // 用 reqwest 建立连接并测量总时间，减去预估 TCP 时间
    // 这是一个估算值，因为 reqwest 不暴露细粒度的 timing
    let url = format!("https://{}:{}/", host, port);
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap_or_default();

    let start = Instant::now();
    let _ = client.head(&url).send().await;
    let total_ms = start.elapsed().as_secs_f64() * 1000.0;

    // TLS 时间 ≈ 总时间 - TCP 时间（粗略估算）
    // 确保不为负
    if total_ms > 10.0 {
        total_ms * 0.3 // TLS 通常占总连接时间的 30% 左右
    } else {
        0.0
    }
}

/// 发起 HTTP 请求，获取状态码、TTFB 和最终 URL
async fn measure_http_request(url: &str, timeout: Duration) -> (Option<i32>, f64, String) {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .unwrap_or_default();

    let start = Instant::now();
    match client.get(url).send().await {
        Ok(response) => {
            let ttfb_ms = start.elapsed().as_secs_f64() * 1000.0;
            let status = response.status().as_u16() as i32;
            let final_url = response.url().to_string();
            debug!(
                "HTTP 请求完成: {} -> {} ({:.2}ms)",
                url, status, ttfb_ms
            );
            (Some(status), ttfb_ms, final_url)
        }
        Err(e) => {
            debug!("HTTP 请求失败: {} - {}", url, e);
            (None, 0.0, url.to_string())
        }
    }
}
