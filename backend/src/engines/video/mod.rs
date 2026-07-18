pub mod cdp_handler;
pub mod collector;

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::config::VideoPlatformConfig;
use crate::engines::browser::provider::{BrowserPage, BrowserProvider};
use crate::engines::dns::DnsEngine;
use crate::engines::http::HttpEngine;

/// 视频测试结果（CDP Media Pipeline）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VideoTestResult {
    pub platform: String,
    // network
    pub dns_time_ms: Option<f64>,
    pub dns_success: bool,
    pub tcp_time_ms: Option<f64>,
    pub http_response_ms: Option<f64>,
    // player (CDP Media 域)
    pub player_type: Option<String>,
    pub mime_type: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub resolution: Option<String>,
    pub fps: Option<f64>,
    pub video_bitrate_kbps: Option<f64>,
    pub audio_bitrate_kbps: Option<f64>,
    // quality
    pub first_play_time_ms: Option<f64>,
    pub play_success: bool,
    pub buffer_count: Option<i32>,
    pub buffer_time_ms: Option<f64>,
    pub dropped_frames: Option<i32>,
    pub decoded_frames: Option<i32>,
    // traffic (CDP Network 域)
    pub video_host: Option<String>,
    pub audio_host: Option<String>,
    pub cdn_node: Option<String>,
    pub segment_count: Option<i32>,
    pub total_bytes: Option<i32>,
    pub download_speed: Option<f64>,
    pub avg_speed: Option<f64>,
    pub peak_speed: Option<f64>,
    // legacy (兼容旧字段)
    pub total_buffer_time_ms: Option<f64>,
    pub buffer_rate: Option<f64>,
    pub video_download_speed: Option<f64>,
    pub video_size: Option<i32>,
    pub video_duration_ms: Option<f64>,
    pub page_title: Option<String>,
    pub screenshot: Option<Vec<u8>>,
    pub error: Option<String>,
}

/// 视频测试引擎 — 通过 BrowserProvider trait
pub struct VideoEngine {
    provider: Arc<Box<dyn BrowserProvider>>,
    chrome_path: String,
    headless: bool,
    timeout: Duration,
    play_duration: Duration,
}

impl VideoEngine {
    pub fn new(
        provider: Arc<Box<dyn BrowserProvider>>,
        chrome_path: &str,
        headless: bool,
        timeout: Duration,
    ) -> Self {
        Self {
            provider,
            chrome_path: chrome_path.to_string(),
            headless,
            timeout,
            play_duration: Duration::from_secs(15),
        }
    }

    /// 测试视频页面（CDP Media Pipeline）
    pub async fn test_page(&self, url: &str, platform_cfg: &VideoPlatformConfig) -> VideoTestResult {
        info!("视频测试开始: {} (平台: {})", url, platform_cfg.name);

        // 1. DNS 探测
        let dns_result = DnsEngine::resolve(url).await.unwrap_or_else(|_| {
            crate::engines::dns::DnsResult { dns_time_ms: 0.0, dns_success: false, resolved_ips: vec![] }
        });

        // 2. HTTP/TCP 探测
        let http_result = HttpEngine::probe(url, self.timeout).await;

        // 3. 启动浏览器 + 启用 CDP 域
        let handle = match self.provider.launch(self.chrome_path.clone(), self.headless, vec![]).await {
            Ok(h) => h,
            Err(e) => return video_err(&platform_cfg.name, &e.to_string(), dns_result, http_result),
        };
        let page = match handle.new_page().await {
            Ok(p) => p,
            Err(e) => return video_err(&platform_cfg.name, &e.to_string(), dns_result, http_result),
        };

        // 启用 CDP 域
        let _ = page.send_cdp("Media.enable", serde_json::json!({}));
        let _ = page.send_cdp("Network.enable", serde_json::json!({}));
        let _ = page.send_cdp("Performance.enable", serde_json::json!({"timeDomain":"timeTicks"}));

        // 注册事件采集器
        let cdp_handler = Arc::new(cdp_handler::VideoCdpHandler::new());
        page.on_cdp_event(cdp_handler.clone());

        // 4. 导航到页面
        if let Err(e) = page.navigate_to(url).await {
            return video_err(&platform_cfg.name, &e.to_string(), dns_result, http_result);
        }
        if let Err(e) = page.wait_for_load().await {
            debug!("等待导航完成: {}", e);
        }

        // 5. 等待播放（detect_only 时等短一些）
        let wait = if platform_cfg.is_detect_only() {
            Duration::from_secs(platform_cfg.wait_seconds.unwrap_or(2))
        } else {
            Duration::from_secs(platform_cfg.wait_seconds.unwrap_or(3)) + self.play_duration
        };
        tokio::time::sleep(wait).await;

        // 6. 采集结果
        let page_title = page.evaluate_sync("document.title").ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let screenshot = page.screenshot().ok();

        let media = cdp_handler.media.snapshot();
        let net = cdp_handler.network.snapshot();

        // 仅检测可访问性
        if platform_cfg.is_detect_only() {
            return VideoTestResult {
                platform: platform_cfg.name.clone(),
                dns_time_ms: Some(dns_result.dns_time_ms),
                dns_success: dns_result.dns_success,
                tcp_time_ms: Some(http_result.tcp_time_ms),
                http_response_ms: Some(http_result.ttfb_ms),
                play_success: true,
                page_title,
                screenshot,
                error: None,
                ..Default::default()
            };
        }

        debug!("视频测试完成: {} (平台:{}) - 播放:{} 缓冲:{}次 seg:{}",
            url, platform_cfg.name, media.play_success, media.buffer_count, net.segment_count);

        VideoTestResult {
            platform: platform_cfg.name.clone(),
            dns_time_ms: Some(dns_result.dns_time_ms),
            dns_success: dns_result.dns_success,
            tcp_time_ms: Some(http_result.tcp_time_ms),
            http_response_ms: Some(http_result.ttfb_ms),
            // CDP 采集
            player_type: media.player_type,
            mime_type: media.mime_type,
            video_codec: media.video_codec,
            audio_codec: media.audio_codec,
            resolution: media.resolution,
            fps: media.fps,
            video_bitrate_kbps: media.video_bitrate_kbps,
            audio_bitrate_kbps: media.audio_bitrate_kbps,
            first_play_time_ms: media.first_play_time_ms,
            play_success: media.play_success,
            buffer_count: Some(media.buffer_count),
            buffer_time_ms: Some(media.buffer_time_ms),
            dropped_frames: Some(media.dropped_frames),
            decoded_frames: Some(media.decoded_frames),
            // Network 采集
            video_host: net.video_host,
            audio_host: net.audio_host,
            cdn_node: net.cdn_node,
            segment_count: Some(net.segment_count),
            total_bytes: Some(net.total_bytes as i32),
            download_speed: net.download_speed,
            avg_speed: net.avg_speed,
            peak_speed: net.peak_speed,
            // 传统
            page_title,
            screenshot,
            error: if !media.play_success && media.first_play_time_ms.is_none() {
                Some("视频未检测到播放事件（CDP Media 域无 playing 事件）".to_string())
            } else { None },
            ..Default::default()
        }
    }
}

fn video_err(
    platform: &str, msg: &str,
    dns: crate::engines::dns::DnsResult,
    http: crate::engines::http::HttpResult,
) -> VideoTestResult {
    VideoTestResult {
        platform: platform.to_string(),
        dns_time_ms: Some(dns.dns_time_ms), dns_success: dns.dns_success,
        tcp_time_ms: Some(http.tcp_time_ms),
        http_response_ms: Some(http.ttfb_ms),
        play_success: false,
        error: Some(msg.to_string()),
        ..Default::default()
    }
}

