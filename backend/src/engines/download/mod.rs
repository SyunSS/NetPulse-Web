use std::time::{Duration, Instant};

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// 下载测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTestResult {
    pub download_speed: f64,       // 当前速度 KB/s
    pub avg_speed: f64,            // 平均速度 KB/s
    pub peak_speed: f64,           // 峰值速度 KB/s
    pub download_time_ms: f64,     // 下载耗时 ms
    pub file_size: i32,            // 已下载大小 bytes
    pub success: bool,
    pub error: Option<String>,
}

/// 下载引擎 — 基于 reqwest streaming + 分段计速
pub struct DownloadEngine {
    timeout: Duration,
    max_duration: Duration,
}

impl DownloadEngine {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            max_duration: Duration::from_secs(15),
        }
    }

    /// 执行下载测试
    pub async fn test_download(&self, url: &str) -> DownloadTestResult {
        info!("下载测试开始: {}", url);

        let client = match reqwest::Client::builder()
            .timeout(self.timeout)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
        {
            Ok(c) => c,
            Err(e) => return error_result(&format!("创建客户端失败: {}", e)),
        };

        let start = Instant::now();

        let response = match client.get(url).send().await {
            Ok(r) => r,
            Err(e) => return error_result(&format!("请求失败: {}", e)),
        };

        if !response.status().is_success() {
            return error_result(&format!("HTTP {}", response.status().as_u16()));
        }

        // 流式下载 + 分段计速
        let mut total_bytes = 0u64;
        let mut peak_speed = 0.0f64;
        let mut speed_samples: Vec<f64> = Vec::new();
        let mut segment_bytes = 0u64;
        let mut last_report = Instant::now();
        let download_start = Instant::now();
        let mut stream = response.bytes_stream();
        let mut stalled_start: Option<Instant> = None;

        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(e) => {
                    warn!("下载流错误: {}", e);
                    break;
                }
            };

            let len = chunk.len() as u64;
            total_bytes += len;
            segment_bytes += len;
            stalled_start = None;

            // 每秒统计一次速度
            let elapsed = last_report.elapsed();
            if elapsed >= Duration::from_secs(1) {
                let speed = segment_bytes as f64 / elapsed.as_secs_f64() / 1024.0;
                speed_samples.push(speed);
                if speed > peak_speed {
                    peak_speed = speed;
                }
                segment_bytes = 0;
                last_report = Instant::now();
            }

            // 检查最大下载时长
            if download_start.elapsed() >= self.max_duration {
                debug!("达到最大下载时长 {}s，停止", self.max_duration.as_secs());
                break;
            }
        }

        // 处理剩余未统计的数据
        if segment_bytes > 0 {
            let elapsed = last_report.elapsed().as_secs_f64();
            if elapsed > 0.01 {
                let speed = segment_bytes as f64 / elapsed / 1024.0;
                speed_samples.push(speed);
                if speed > peak_speed {
                    peak_speed = speed;
                }
            }
        }

        let elapsed_ms = download_start.elapsed().as_secs_f64() * 1000.0;

        let avg_speed = if elapsed_ms > 0.0 && total_bytes > 0 {
            total_bytes as f64 / (elapsed_ms / 1000.0) / 1024.0
        } else {
            0.0
        };

        let current_speed = speed_samples.last().copied().unwrap_or(avg_speed);

        debug!(
            "下载完成: {} - {:.1}KB/s avg, {:.1}KB/s peak, {} bytes",
            url, avg_speed, peak_speed, total_bytes
        );

        DownloadTestResult {
            download_speed: current_speed,
            avg_speed,
            peak_speed,
            download_time_ms: elapsed_ms,
            file_size: total_bytes as i32,
            success: true,
            error: None,
        }
    }
}

fn error_result(msg: &str) -> DownloadTestResult {
    DownloadTestResult {
        download_speed: 0.0,
        avg_speed: 0.0,
        peak_speed: 0.0,
        download_time_ms: 0.0,
        file_size: 0,
        success: false,
        error: Some(msg.to_string()),
    }
}
