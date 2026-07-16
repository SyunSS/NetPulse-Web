use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::config::VideoPlatformConfig;
use crate::engines::browser::provider::{BrowserPage, BrowserProvider};
use crate::engines::dns::DnsEngine;
use crate::engines::http::HttpEngine;

/// 视频测试结果
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VideoTestResult {
    pub platform: String,
    pub dns_time_ms: Option<f64>,
    pub dns_success: bool,
    pub tcp_time_ms: Option<f64>,
    pub http_response_ms: Option<f64>,
    pub first_play_time_ms: Option<f64>,
    pub buffer_count: Option<i32>,
    pub total_buffer_time_ms: Option<f64>,
    pub buffer_rate: Option<f64>,
    pub play_success: bool,
    pub video_download_speed: Option<f64>,
    pub video_size: Option<i32>,
    pub video_duration_ms: Option<f64>,
    pub dropped_frames: Option<i32>,
    pub decoded_frames: Option<i32>,
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

    /// 测试视频页面
    pub async fn test_page(&self, url: &str, platform_cfg: &VideoPlatformConfig) -> VideoTestResult {
        info!("视频测试开始: {} (平台: {})", url, platform_cfg.name);

        // 1. DNS 探测
        let dns_result = DnsEngine::resolve(url).await.unwrap_or_else(|_| {
            crate::engines::dns::DnsResult { dns_time_ms: 0.0, dns_success: false, resolved_ips: vec![] }
        });

        // 2. HTTP/TCP 探测
        let http_result = HttpEngine::probe(url, self.timeout).await;

        // 3. 浏览器探测
        let extra_args = if platform_cfg.is_detect_only() {
            vec![]
        } else {
            vec!["--autoplay-policy=no-user-gesture-required".to_string(),
                 "--mute-audio".to_string()]
        };

        let handle = match self.provider.launch(self.chrome_path.clone(), self.headless, extra_args).await {
            Ok(h) => h,
            Err(e) => return video_err(&platform_cfg.name, &e.to_string(), dns_result, http_result),
        };

        let page = match handle.new_page().await {
            Ok(p) => p,
            Err(e) => return video_err(&platform_cfg.name, &e.to_string(), dns_result, http_result),
        };

        if let Err(e) = page.navigate_to(url).await {
            return video_err(&platform_cfg.name, &e.to_string(), dns_result, http_result);
        }
        if let Err(e) = page.wait_for_load().await {
            debug!("等待导航完成: {}", e);
        }

        // 仅检测可访问性
        if platform_cfg.is_detect_only() {
            let title = page.evaluate_sync("document.title").ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()));
            let accessible = true; // 已成功导航

            return VideoTestResult {
                platform: platform_cfg.name.clone(),
                dns_time_ms: Some(dns_result.dns_time_ms),
                dns_success: dns_result.dns_success,
                tcp_time_ms: Some(http_result.tcp_time_ms),
                http_response_ms: Some(http_result.ttfb_ms),
                play_success: accessible,
                page_title: title,
                error: None,
                ..Default::default()
            };
        }

        // 完整视频播放检测
        let wait_seconds = platform_cfg.wait_seconds.unwrap_or(1);
        let video_selector = platform_cfg.video_selector.clone().unwrap_or_else(|| "video".to_string());

        tokio::time::sleep(Duration::from_secs(wait_seconds)).await;

        // 注入监听 JS — 改为同步 IIFE，先设置监听器再尝试播放
        let inject_js = build_inject_js_sync(&video_selector);
        let _ = page.evaluate_sync(&inject_js);

        // 等待播放
        tokio::time::sleep(self.play_duration).await;

        // 采集最终结果 — 同步 JS
        let collect_js = build_collect_js_sync(&video_selector);
        let final_data: VideoJsData = page.evaluate_sync(&collect_js)
            .ok()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let title = page.evaluate_sync("document.title").ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        let screenshot = page.screenshot().ok();

        debug!("视频测试完成: {} (平台:{}) - 播放:{} 缓冲:{}次",
            url, platform_cfg.name, final_data.play_success, final_data.buffer_count);

        VideoTestResult {
            platform: platform_cfg.name.clone(),
            dns_time_ms: Some(dns_result.dns_time_ms),
            dns_success: dns_result.dns_success,
            tcp_time_ms: Some(http_result.tcp_time_ms),
            http_response_ms: Some(http_result.ttfb_ms),
            first_play_time_ms: final_data.first_play_time_ms,
            buffer_count: Some(final_data.buffer_count),
            total_buffer_time_ms: Some(final_data.total_buffer_time_ms),
            play_success: final_data.play_success,
            video_download_speed: final_data.video_download_speed,
            video_size: final_data.video_size,
            video_duration_ms: final_data.video_duration_ms,
            dropped_frames: Some(final_data.dropped_frames),
            decoded_frames: Some(final_data.decoded_frames),
            page_title: title,
            screenshot,
            error: if !final_data.play_success && final_data.first_play_time_ms.is_none() {
                Some("视频未检测到播放事件（页面已加载）".to_string())
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

// ─── JS 注入（同步 IIFE）─────────────────────────────

/// 注入视频监听 — 同步 IIFE，不依赖 async evaluate
fn build_inject_js_sync(video_selector: &str) -> String {
    format!(
        r#"
(function() {{
    var selectors = ['{0}', 'video', '.video video', '#video', 'video.html5-main-video'];
    var video = null;
    for (var i=0; i<selectors.length; i++) {{
        var el = document.querySelector(selectors[i]);
        if (el && el.tagName === 'VIDEO') {{ video = el; break; }}
    }}
    if (!video) {{
        var all = document.querySelectorAll('video');
        if (all.length > 0) video = all[0];
    }}
    if (!video) {{
        window.__videoStats = {{ play_success: false, error: 'no_video_element' }};
        return;
    }}

    window.__videoStats = {{
        first_play_time_ms: null, buffer_count: 0, total_buffer_time_ms: 0,
        play_success: false, video_download_speed: null, video_size: null,
        video_duration_ms: null, dropped_frames: 0, decoded_frames: 0,
        _buffer_start: null, _start_time: performance.now()
    }};

    video.addEventListener('playing', function() {{
        var s = window.__videoStats;
        if (s.first_play_time_ms === null) {{
            s.first_play_time_ms = performance.now() - s._start_time;
            s.play_success = true;
        }}
        if (s._buffer_start !== null) {{
            s.total_buffer_time_ms += performance.now() - s._buffer_start;
            s._buffer_start = null;
        }}
    }});

    video.addEventListener('waiting', function() {{
        window.__videoStats.buffer_count++;
        window.__videoStats._buffer_start = performance.now();
    }});

    video.addEventListener('loadedmetadata', function() {{
        window.__videoStats.video_duration_ms = video.duration ? video.duration * 1000 : null;
    }});

    // 尝试播放
    try {{ video.play(); }} catch(e) {{}}
}})()
"#,
        video_selector.replace('\'', "\\'")
    )
}

/// 采集最终结果 — 同步查询
fn build_collect_js_sync(video_selector: &str) -> String {
    format!(
        r#"
(function() {{
    if (!window.__videoStats) return {{ play_success: false }};
    var s = window.__videoStats;

    var video = null;
    var selectors = ['{0}', 'video', '.video video', '#video', 'video.html5-main-video'];
    for (var i=0; i<selectors.length; i++) {{
        var el = document.querySelector(selectors[i]);
        if (el && el.tagName === 'VIDEO') {{ video = el; break; }}
    }}
    if (!video) {{
        var all = document.querySelectorAll('video');
        if (all.length > 0) video = all[0];
    }}

    if (video) {{
        if (!s.play_success && !video.paused && video.currentTime > 0) {{
            s.first_play_time_ms = performance.now() - s._start_time;
            s.play_success = true;
        }}
        if (video.duration && !s.video_duration_ms) {{
            s.video_duration_ms = video.duration * 1000;
        }}
        if (video.webkitDecodedFrameCount !== undefined) s.decoded_frames = video.webkitDecodedFrameCount;
        if (video.webkitDroppedFrameCount !== undefined) s.dropped_frames = video.webkitDroppedFrameCount;
        if (!s.play_success && video.currentTime > 0.5) {{
            s.first_play_time_ms = performance.now() - s._start_time;
            s.play_success = true;
        }}
    }}

    // 采集资源大小
    var resources = performance.getEntriesByType('resource');
    var videoSize = 0;
    for (var i=0; i<resources.length; i++) {{
        if (resources[i].initiatorType === 'video' ||
            resources[i].name.indexOf('.mp4') !== -1 || resources[i].name.indexOf('.m3u8') !== -1) {{
            videoSize += resources[i].transferSize || 0;
        }}
    }}
    s.video_size = videoSize || null;

    if (s.video_size && s.first_play_time_ms) {{
        s.video_download_speed = (s.video_size / 1024) / (s.first_play_time_ms / 1000);
    }}

    return {{
        first_play_time_ms: s.first_play_time_ms,
        buffer_count: s.buffer_count,
        total_buffer_time_ms: s.total_buffer_time_ms,
        play_success: s.play_success,
        video_download_speed: s.video_download_speed,
        video_size: s.video_size,
        video_duration_ms: s.video_duration_ms,
        dropped_frames: s.dropped_frames,
        decoded_frames: s.decoded_frames
    }};
}})()
"#,
        video_selector.replace('\'', "\\'")
    )
}

#[derive(Debug, Default, Deserialize)]
struct VideoJsData {
    #[serde(default)]
    first_play_time_ms: Option<f64>,
    #[serde(default)]
    buffer_count: i32,
    #[serde(default)]
    total_buffer_time_ms: f64,
    #[serde(default)]
    play_success: bool,
    #[serde(default)]
    video_download_speed: Option<f64>,
    #[serde(default)]
    video_size: Option<i32>,
    #[serde(default)]
    video_duration_ms: Option<f64>,
    #[serde(default)]
    dropped_frames: i32,
    #[serde(default)]
    decoded_frames: i32,
}
