pub mod cdp_handler;
pub mod collector;
pub mod playback;

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
    // diagnostics
    pub trigger_method: Option<String>,
    pub player_created: Option<bool>,
    pub media_events: Option<i32>,
    pub network_media_requests: Option<i32>,
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

        // 3. 启动浏览器 (autoplay + mute + disable media engagement)
        let launch_args = vec![
            "--autoplay-policy=no-user-gesture-required".into(),
            "--mute-audio".into(),
            "--disable-features=PreloadMediaEngagementData,MediaEngagementBypassAutoplayPolicies".into(),
        ];
        let handle = match self.provider.launch(self.chrome_path.clone(), self.headless, launch_args).await {
            Ok(h) => h,
            Err(e) => return video_err(&platform_cfg.name, &e.to_string(), dns_result, http_result),
        };
        let page = match handle.new_page().await {
            Ok(p) => p,
            Err(e) => return video_err(&platform_cfg.name, &e.to_string(), dns_result, http_result),
        };

        // 4. 启用 CDP 域（导航前！）
        let _ = page.send_cdp("Media.enable", serde_json::json!({}));
        let _ = page.send_cdp("Network.enable", serde_json::json!({}));
        let _ = page.send_cdp("Page.enable", serde_json::json!({}));

        // 注册事件采集器
        let cdp_handler = Arc::new(cdp_handler::VideoCdpHandler::new());
        page.on_cdp_event(cdp_handler.clone());

        // 5. 导航
        if let Err(e) = page.navigate_to(url).await {
            return video_err(&platform_cfg.name, &e.to_string(), dns_result, http_result);
        }
        if let Err(e) = page.wait_for_load().await {
            debug!("等待导航完成: {}", e);
        }

        // 6. 页面预处理（关闭 cookie/弹窗）
        page_preprocess(&*page).await;

        // 6.5 注入视频事件监听 JS（轻量，用于补充 first_play / buffer 统计）
        let _ = page.evaluate_sync(
            r#"(function(){
                var v = document.querySelector('video');
                if (!v) return;
                window.__videoStats = window.__videoStats || { first_play_time_ms: null, buffer_count: 0, total_buffer_time_ms: 0, _buffer_start: null, _start: performance.now() };
                var s = window.__videoStats;
                v.addEventListener('playing', function(){ if (s.first_play_time_ms===null) s.first_play_time_ms = performance.now()-s._start; });
                v.addEventListener('waiting', function(){ s.buffer_count++; s._buffer_start = performance.now(); });
                v.addEventListener('playing', function(){ if (s._buffer_start!==null){ s.total_buffer_time_ms += performance.now()-s._buffer_start; s._buffer_start=null; }});
                try { v.play().catch(function(){}); } catch(e){}
            })()"#
        );

        // 7. PlaybackController: 多级播放触发
        let max_wait = if platform_cfg.is_detect_only() { 10 } else { 60 };
        let mut ctrl = playback::PlaybackController::new(max_wait);

        // 模拟点击的闭包
        let click_page = || {
            // 点击页面中心触发播放
            let _ = page.evaluate_sync(
                "document.elementFromPoint(window.innerWidth/2,window.innerHeight/2)?.click()"
            );
        };

        // 轮询检查播放器状态 + 等待
        let check_duration = if platform_cfg.is_detect_only() { Duration::from_secs(3) } else { Duration::from_secs(6) };
        for _ in 0..((max_wait / 2) as usize) {
            tokio::time::sleep(check_duration.min(Duration::from_secs(2))).await;
            // 检查是否有媒体请求
            let net = cdp_handler.network.snapshot();
            if net.segment_count > 0 { ctrl.on_network_media(); }
        }
        let trigger = ctrl.trigger_method();
        let diag = ctrl.diagnostics();

        // 8. JS 直接读取 video 元素实时状态（弥补 CDP 事件不稳定的问题）
        let video_state_js = r#"JSON.stringify((function(){
            var videos = document.querySelectorAll('video');
            if (videos.length === 0) return {count:0};
            var v = videos[0];
            var stats = window.__videoStats || {};
            return {
                count: videos.length, paused: v.paused, currentTime: v.currentTime,
                duration: v.duration, ended: v.ended,
                readyState: v.readyState, networkState: v.networkState,
                webkitDecodedFrameCount: v.webkitDecodedFrameCount,
                webkitDroppedFrameCount: v.webkitDroppedFrameCount,
                first_play_ms: stats.first_play_time_ms || null,
                buffer_count: stats.buffer_count || 0,
                buffer_time: stats.total_buffer_time_ms || 0,
                is_playing: !v.paused && v.currentTime > 0.1
            };
        })())"#;
        let video_js: serde_json::Value = page.evaluate_sync(video_state_js)
            .ok().and_then(|v| v.as_str().and_then(|s| serde_json::from_str(s).ok()))
            .unwrap_or(serde_json::json!({"count":0}));

        // 解析 JS 采集的数据（作为 CDP fallback）
        let js_first_play = video_js.get("first_play_ms").and_then(|v| v.as_f64());
        let js_buffer_count = video_js.get("buffer_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let js_buffer_time = video_js.get("buffer_time").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let js_playing = video_js.get("is_playing").and_then(|v| v.as_bool()).unwrap_or(false);
        let js_dropped = video_js.get("webkitDroppedFrameCount").and_then(|v| v.as_i64());
        let js_decoded = video_js.get("webkitDecodedFrameCount").and_then(|v| v.as_i64());

        // 9. 采集 CDP 结果
        let page_title = page.evaluate_sync("document.title").ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let screenshot = page.screenshot().ok();

        let media = cdp_handler.media.snapshot();
        let net = cdp_handler.network.snapshot();

        // 仅检测可访问性
        if platform_cfg.is_detect_only() {
            return VideoTestResult {
                platform: platform_cfg.name.clone(),
                dns_time_ms: Some(dns_result.dns_time_ms), dns_success: dns_result.dns_success,
                tcp_time_ms: Some(http_result.tcp_time_ms), http_response_ms: Some(http_result.ttfb_ms),
                play_success: true, page_title, screenshot, error: None,
                trigger_method: Some("detect_only".into()),
                ..Default::default()
            };
        }

        // 判断总体播放状态: CDP 事件 + Netzwork segments + JS playing
        let has_playback = media.play_success
            || media.first_play_time_ms.is_some()
            || js_first_play.is_some()
            || diag.media_event_count > 0
            || diag.player_created
            || net.segment_count > 0
            || js_playing;

        let error_msg = if has_playback { None }
            else { Some(format!("无播放: CDP={} net={} seg={} js_playing={}",
                diag.media_event_count, diag.network_media_request_count, net.segment_count, js_playing)) };

        VideoTestResult {
            platform: platform_cfg.name.clone(),
            dns_time_ms: Some(dns_result.dns_time_ms), dns_success: dns_result.dns_success,
            tcp_time_ms: Some(http_result.tcp_time_ms), http_response_ms: Some(http_result.ttfb_ms),
            player_type: media.player_type, mime_type: media.mime_type,
            video_codec: media.video_codec, audio_codec: media.audio_codec,
            resolution: media.resolution, fps: media.fps,
            video_bitrate_kbps: media.video_bitrate_kbps, audio_bitrate_kbps: media.audio_bitrate_kbps,
            first_play_time_ms: media.first_play_time_ms.or(js_first_play),
            play_success: has_playback,
            buffer_count: Some(if media.buffer_count > 0 { media.buffer_count } else { js_buffer_count }),
            buffer_time_ms: Some(if media.buffer_time_ms > 0.0 { media.buffer_time_ms } else { js_buffer_time }),
            dropped_frames: Some(media.dropped_frames.max(js_dropped.unwrap_or(0) as i32)),
            decoded_frames: Some(media.decoded_frames.max(js_decoded.unwrap_or(0) as i32)),
            video_host: net.video_host, audio_host: net.audio_host, cdn_node: net.cdn_node,
            segment_count: Some(net.segment_count), total_bytes: Some(net.total_bytes as i32),
            download_speed: net.download_speed.or_else(|| {
                let total = net.total_bytes as f64;
                if total > 0.0 && diag.total_wait_ms > 0 {
                    Some(total / (diag.total_wait_ms as f64 / 1000.0) / 1024.0)
                } else { None }
            }),
            avg_speed: net.avg_speed, peak_speed: net.peak_speed,
            page_title, screenshot, error: error_msg,
            trigger_method: Some(diag.trigger_method.clone()),
            player_created: Some(diag.player_created),
            media_events: Some(diag.media_event_count as i32),
            network_media_requests: Some(diag.network_media_request_count as i32),
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

/// 页面预处理：关闭常见弹窗 (cookie, 年龄确认等)
async fn page_preprocess(page: &(dyn BrowserPage + Sync)) {
    let js = r#"
(function(){
    var btns = document.querySelectorAll('button,a,[role=button]');
    var keywords = ['accept','agree','allow','ok','yes','同意','接受','允许','确定','继续','关闭','close','dismiss','got it','skip','later','稍后'];
    for (var i=0; i<btns.length; i++) {
        var t = (btns[i].textContent||'').toLowerCase();
        for (var j=0; j<keywords.length; j++) {
            if (t.includes(keywords[j])) { try { btns[i].click(); } catch(e){} break; }
        }
    }
})()
"#;
    let _ = page.evaluate_sync(js);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
}

