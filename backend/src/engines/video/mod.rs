pub mod browser;
pub mod cdp {
    pub mod media;
    pub mod network;
    pub mod page;
    pub mod performance;
    pub mod runtime;
}
pub mod diagnostics;
pub mod events;
pub mod hooks;
pub mod metrics;
pub mod players;

use std::sync::Arc;
use std::time::Duration;

use chromiumoxide::cdp::browser_protocol::network::SetCacheDisabledParams;
use futures::StreamExt;
use tokio::sync::Semaphore;
use tracing::{error, info};

use crate::config::VideoPlatformConfig;
use crate::engines::dns::DnsEngine;
use crate::engines::http::HttpEngine;

use self::diagnostics::DiagnosticLogger;
use self::events::{EventMeta, VideoEvent};
use self::metrics::{MetricCollector, VideoMetrics};
use self::players::registry::PlayerRegistry;

/// 视频测试结果（保持与旧 VideoTestResult 字段兼容）
#[derive(Debug, Clone)]
pub struct VideoTestResult {
    pub platform: String,
    pub dns_time_ms: Option<f64>,
    pub dns_success: bool,
    pub tcp_time_ms: Option<f64>,
    pub http_response_ms: Option<f64>,
    pub first_play_time_ms: Option<f64>,
    pub play_success: bool,
    pub buffer_count: Option<i32>,
    pub buffer_time_ms: Option<f64>,
    pub total_buffer_time_ms: Option<f64>,
    pub buffer_rate: Option<f64>,
    pub dropped_frames: Option<i32>,
    pub decoded_frames: Option<i32>,
    pub video_download_speed: Option<f64>,
    pub video_size: Option<i32>,
    pub video_duration_ms: Option<f64>,
    pub video_host: Option<String>,
    pub page_title: Option<String>,
    pub screenshot: Option<Vec<u8>>,
    pub error: Option<String>,
    pub trigger_method: Option<String>,
    pub stutter_count: Option<i32>,
    pub stutter_duration_ms: Option<f64>,
    pub play_duration_sec: Option<f64>,
    pub stutter_ratio: Option<f64>,
    pub video_width: Option<i32>,
    pub video_height: Option<i32>,
    pub video_duration_sec: Option<f64>,
    pub player_type: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub resolution: Option<String>,
    pub fps: Option<f64>,
    pub video_bitrate_kbps: Option<f64>,
    pub audio_bitrate_kbps: Option<f64>,
    pub segment_count: Option<i32>,
    pub total_bytes: Option<i32>,
    pub download_speed: Option<f64>,
    pub peak_speed: Option<f64>,
}

impl Default for VideoTestResult {
    fn default() -> Self {
        Self {
            platform: String::new(),
            dns_time_ms: None,
            dns_success: false,
            tcp_time_ms: None,
            http_response_ms: None,
            first_play_time_ms: None,
            play_success: false,
            buffer_count: None,
            buffer_time_ms: None,
            total_buffer_time_ms: None,
            buffer_rate: None,
            dropped_frames: None,
            decoded_frames: None,
            video_download_speed: None,
            video_size: None,
            video_duration_ms: None,
            video_host: None,
            page_title: None,
            screenshot: None,
            error: None,
            trigger_method: None,
            stutter_count: None,
            stutter_duration_ms: None,
            play_duration_sec: None,
            stutter_ratio: None,
            video_width: None,
            video_height: None,
            video_duration_sec: None,
            player_type: None,
            video_codec: None,
            audio_codec: None,
            resolution: None,
            fps: None,
            video_bitrate_kbps: None,
            audio_bitrate_kbps: None,
            segment_count: None,
            total_bytes: None,
            download_speed: None,
            peak_speed: None,
        }
    }
}

impl From<VideoMetrics> for VideoTestResult {
    fn from(m: VideoMetrics) -> Self {
        VideoTestResult {
            platform: m.platform,
            dns_time_ms: Some(m.dns_time_ms),
            dns_success: m.dns_success,
            tcp_time_ms: Some(m.tcp_time_ms),
            http_response_ms: Some(m.http_response_ms),
            first_play_time_ms: m.first_play_time_ms,
            play_success: m.play_success,
            buffer_count: Some(m.buffer_count as i32),
            buffer_time_ms: Some(m.buffer_time_ms),
            total_buffer_time_ms: Some(m.buffer_time_ms),
            buffer_rate: None,
            dropped_frames: Some(m.dropped_frames as i32),
            decoded_frames: Some(m.decoded_frames as i32),
            video_download_speed: m.download_speed,
            video_size: Some(m.total_bytes.min(i32::MAX as u64) as i32),
            video_duration_ms: Some(m.video_duration_sec * 1000.0),
            video_host: m.video_host,
            page_title: m.page_title,
            screenshot: m.screenshot,
            error: m.error,
            trigger_method: Some(m.trigger_method),
            stutter_count: Some(m.stutter_count as i32),
            stutter_duration_ms: Some(m.stutter_duration_ms),
            play_duration_sec: Some(m.play_duration_sec),
            stutter_ratio: Some(m.stutter_ratio),
            video_width: Some(m.video_width as i32),
            video_height: Some(m.video_height as i32),
            video_duration_sec: Some(m.video_duration_sec),
            player_type: m.player_type,
            video_codec: m.video_codec,
            audio_codec: m.audio_codec,
            resolution: m.resolution,
            fps: m.fps,
            video_bitrate_kbps: m.video_bitrate_kbps,
            audio_bitrate_kbps: m.audio_bitrate_kbps,
            segment_count: Some(m.segment_count as i32),
            total_bytes: Some(m.total_bytes.min(i32::MAX as u64) as i32),
            download_speed: m.download_speed,
            peak_speed: m.peak_speed,
        }
    }
}

pub struct VideoEngine {
    chrome_path: String,
    headless: bool,
    user_data_dir: Option<String>,
    timeout: Duration,
    play_duration: Duration,
    browser_semaphore: Arc<Semaphore>,
}

impl VideoEngine {
    pub fn new(
        chrome_path: &str,
        headless: bool,
        user_data_dir: Option<String>,
        timeout: Duration,
        browser_semaphore: Arc<Semaphore>,
    ) -> Self {
        Self {
            chrome_path: chrome_path.to_string(),
            headless,
            user_data_dir,
            timeout,
            play_duration: Duration::from_secs(60),
            browser_semaphore,
        }
    }

    pub async fn test_page(
        &self,
        url: &str,
        platform_cfg: &VideoPlatformConfig,
        cookie_json: Option<serde_json::Value>,
    ) -> VideoTestResult {
        self.test_page_inner(url, platform_cfg, cookie_json).await
    }

    async fn test_page_inner(
        &self,
        url: &str,
        platform_cfg: &VideoPlatformConfig,
        cookie_json: Option<serde_json::Value>,
    ) -> VideoTestResult {
        let diag = DiagnosticLogger::new();
        info!(
            "[VideoEngine] 开始测试: {} (平台: {})",
            url, platform_cfg.name
        );
        diag.log_phase(&format!("开始测试: {}", url));

        // 1. DNS 探测
        diag.log_phase("DNS 解析...");
        let dns_result =
            DnsEngine::resolve(url)
                .await
                .unwrap_or_else(|_| crate::engines::dns::DnsResult {
                    dns_time_ms: 0.0,
                    dns_success: false,
                    resolved_ips: vec![],
                });

        // 2. HTTP/TCP 探测
        diag.log_phase("HTTP/TCP 探测...");
        let http_result = HttpEngine::probe(url, self.timeout).await;

        // 仅检测模式
        if platform_cfg.is_detect_only() {
            diag.log_phase("detect_only 模式，跳过浏览器测试");
            let mut result = VideoTestResult {
                platform: platform_cfg.name.clone(),
                dns_time_ms: Some(dns_result.dns_time_ms),
                dns_success: dns_result.dns_success,
                tcp_time_ms: Some(http_result.tcp_time_ms),
                http_response_ms: Some(http_result.ttfb_ms),
                play_success: http_result.error.is_none(),
                trigger_method: Some("detect_only".into()),
                ..Default::default()
            };
            result.error = http_result.error;
            return result;
        }

        // 3. 启动 Chromium
        info!("等待浏览器槽位: type=video url={}", url);
        let wait_start = std::time::Instant::now();
        let permit = match self.browser_semaphore.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(error) => {
                let err_msg = format!("浏览器槽位不可用: {error}");
                error!("{}", err_msg);
                return VideoMetrics::error_result(
                    &platform_cfg.name,
                    dns_result,
                    http_result,
                    &err_msg,
                )
                .into();
            }
        };
        info!(
            "获得浏览器槽位: type=video url={} wait_ms={:.0}",
            url,
            wait_start.elapsed().as_secs_f64() * 1000.0
        );
        let browser_started = std::time::Instant::now();
        diag.log_phase("启动 Chromium...");
        let video_browser_config = crate::config::VideoBrowserConfig {
            path: self.chrome_path.clone(),
            headless: self.headless,
            user_data_dir: self.user_data_dir.clone(),
        };
        let chrome = match browser::ChromiumoxideBrowser::launch(&video_browser_config).await {
            Ok(b) => {
                diag.log_phase("Chromium 启动成功");
                b
            }
            Err(e) => {
                let err_msg = format!("Chromium 启动失败: {:#}", e);
                diag.log_phase(&err_msg);
                error!("{}", err_msg);
                info!(
                    "释放浏览器槽位: type=video url={} held_ms={:.0}",
                    url,
                    browser_started.elapsed().as_secs_f64() * 1000.0
                );
                drop(permit);
                return VideoMetrics::error_result(
                    &platform_cfg.name,
                    dns_result,
                    http_result,
                    &err_msg,
                )
                .into();
            }
        };

        // 4. 创建页面
        diag.log_phase("创建页面...");
        let page = match chrome.new_page().await {
            Ok(p) => p,
            Err(e) => {
                let err_msg = format!("创建页面失败: {:#}", e);
                diag.log_phase(&err_msg);
                error!("{}", err_msg);
                chrome.shutdown().await;
                info!(
                    "释放浏览器槽位: type=video url={} held_ms={:.0}",
                    url,
                    browser_started.elapsed().as_secs_f64() * 1000.0
                );
                drop(permit);
                return VideoMetrics::error_result(
                    &platform_cfg.name,
                    dns_result,
                    http_result,
                    &err_msg,
                )
                .into();
            }
        };
        page.enable_stealth_mode().await.ok();

        // 5. 事件通道
        let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<VideoEvent>();

        let _ = event_tx.send(VideoEvent::ChromiumStarted {
            pid: 0,
            meta: EventMeta::now(),
        });

        // 6. 注册 CDP 事件监听 (Media + Network + Runtime + Page)
        // 注意: 事件监听在导航前注册以确保捕获所有事件
        diag.log_phase("注册 CDP 事件监听器...");
        page.execute(chromiumoxide::cdp::browser_protocol::network::EnableParams::default())
            .await
            .ok();
        page.execute(SetCacheDisabledParams::new(true)).await.ok();
        page.execute(chromiumoxide::cdp::browser_protocol::media::EnableParams::default())
            .await
            .ok();

        let tx = event_tx.clone();
        if let Ok(mut stream) = page
            .event_listener::<chromiumoxide::cdp::browser_protocol::media::EventPlayerCreated>()
            .await
        {
            tokio::spawn(async move {
                let collector = cdp::media::MediaCollector::new(tx.clone());
                while let Some(event) = stream.next().await {
                    collector.handle_player_created(event.as_ref().clone());
                }
            });
        }

        let tx = event_tx.clone();
        if let Ok(mut stream) = page
            .event_listener::<chromiumoxide::cdp::browser_protocol::media::EventPlayerEventsAdded>()
            .await
        {
            tokio::spawn(async move {
                let collector = cdp::media::MediaCollector::new(tx.clone());
                while let Some(event) = stream.next().await {
                    collector.handle_events_added(event.as_ref().clone());
                }
            });
        }

        let tx = event_tx.clone();
        if let Ok(mut stream) = page.event_listener::<
            chromiumoxide::cdp::browser_protocol::media::EventPlayerPropertiesChanged
        >().await {
            tokio::spawn(async move {
                let collector = cdp::media::MediaCollector::new(tx.clone());
                while let Some(event) = stream.next().await {
                    collector.handle_properties_changed(event.as_ref().clone());
                }
            });
        }

        let network_collector = Arc::new(cdp::network::NetworkCollector::new(event_tx.clone()));

        let collector = network_collector.clone();
        if let Ok(mut stream) = page.event_listener::<
            chromiumoxide::cdp::browser_protocol::network::EventRequestWillBeSent
        >().await {
            tokio::spawn(async move {
                while let Some(event) = stream.next().await {
                    collector.handle_request_will_be_sent(event.as_ref().clone());
                }
            });
        }

        let collector = network_collector.clone();
        if let Ok(mut stream) = page
            .event_listener::<chromiumoxide::cdp::browser_protocol::network::EventResponseReceived>(
            )
            .await
        {
            tokio::spawn(async move {
                while let Some(event) = stream.next().await {
                    collector.handle_response_received(event.as_ref().clone());
                }
            });
        }

        let collector = network_collector.clone();
        if let Ok(mut stream) = page
            .event_listener::<chromiumoxide::cdp::browser_protocol::network::EventDataReceived>()
            .await
        {
            tokio::spawn(async move {
                while let Some(event) = stream.next().await {
                    collector.handle_data_received(event.as_ref().clone());
                }
            });
        }

        let tx = event_tx.clone();
        if let Ok(mut stream) = page
            .event_listener::<chromiumoxide::cdp::js_protocol::runtime::EventConsoleApiCalled>()
            .await
        {
            tokio::spawn(async move {
                let collector = cdp::runtime::RuntimeCollector::new(tx.clone());
                while let Some(event) = stream.next().await {
                    collector.handle_console_api_called(event.as_ref().clone());
                }
            });
        }

        if let Ok(mut stream) = page
            .event_listener::<chromiumoxide::cdp::browser_protocol::page::EventLoadEventFired>()
            .await
        {
            tokio::spawn(async move {
                while let Some(_event) = stream.next().await {
                    info!("页面 Load 事件触发");
                }
            });
        }

        diag.log_phase("CDP 事件监听器已注册");

        // 在导航前注册，确保页面脚本创建播放器时 Hook 已存在。
        let hook_manager = hooks::JSHookManager::new(page.clone());
        diag.log_phase("注册导航前 JS Hooks...");
        hook_manager.inject_on_new_document().await.ok();

        if let Some(cookies) = cookie_json.as_ref() {
            match hook_manager.set_cookies_from_json(cookies).await {
                Ok(count) if count > 0 => {
                    diag.log_phase(&format!("已注入平台 Cookie: {} 个", count))
                }
                Ok(_) => diag.log_phase("未发现可注入 Cookie"),
                Err(e) => diag.log_phase(&format!("Cookie 注入失败，继续匿名测试: {}", e)),
            }
        }

        // 7. 导航
        diag.log_phase(&format!("导航: {}", url));
        if let Err(e) = page.goto(url).await {
            let err_msg = format!("导航失败: {:#}", e);
            diag.log_phase(&format!("{}，检查页面是否已可用", err_msg));
            tokio::time::sleep(Duration::from_secs(2)).await;
            let usable = hook_manager.page_is_usable().await.unwrap_or(false);
            if !usable {
                error!("{}", err_msg);
                chrome.shutdown().await;
                info!(
                    "释放浏览器槽位: type=video url={} held_ms={:.0}",
                    url,
                    browser_started.elapsed().as_secs_f64() * 1000.0
                );
                drop(permit);
                return VideoMetrics::error_result(
                    &platform_cfg.name,
                    dns_result,
                    http_result,
                    &err_msg,
                )
                .into();
            }
            diag.log_phase("导航失败但 DOM 可用，继续播放器测试");
        }
        let _ = event_tx.send(VideoEvent::PageLoaded {
            url: url.to_string(),
            final_url: None,
            meta: EventMeta::now(),
        });

        // 8. 等待页面稳定
        tokio::time::sleep(Duration::from_secs(2)).await;

        // 9. 注入 Hook
        hook_manager.dismiss_popups().await.ok();
        tokio::time::sleep(Duration::from_millis(500)).await;

        diag.log_phase("注入 JS Hooks...");
        hook_manager.inject_all().await.ok();
        let _ = event_tx.send(VideoEvent::HooksInjected {
            hook_count: 4,
            meta: EventMeta::now(),
        });

        // 10. 检测播放器
        diag.log_phase("识别播放器...");
        let mut video_count = hook_manager.detect_video_elements().await.unwrap_or(0);
        for _ in 0..8 {
            if video_count > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            video_count = hook_manager.detect_video_elements().await.unwrap_or(0);
        }
        let registry = PlayerRegistry::new();
        let player = registry.detect(&page, url).await;
        let (player_name, play_js, play_selectors) = if let Some(p) = player {
            (
                p.name().to_string(),
                p.play_trigger_js(),
                p.play_button_selectors(),
            )
        } else {
            (
                "html5".to_string(),
                None,
                vec![
                    "video".to_string(),
                    ".play-btn".to_string(),
                    ".btn-play".to_string(),
                ],
            )
        };
        diag.log_phase(&format!("播放器: {}", player_name));

        let _ = event_tx.send(VideoEvent::VideoElementDiscovered {
            selector: "video".into(),
            count: video_count,
            meta: EventMeta::now(),
        });
        info!("发现 {} 个 video 元素", video_count);

        // 在触发播放前初始化收集器，否则播放事件会先进入队列但无法形成指标。
        let mut collector = MetricCollector::new(&platform_cfg.name, &dns_result, &http_result);
        collector.update_trigger_method(&player_name);

        // 11. 触发播放
        diag.log_phase("触发播放...");
        collector.mark_play_requested();
        let clicked = hook_manager
            .click_play_candidate(&play_selectors)
            .await
            .unwrap_or(false);
        if clicked {
            diag.log_phase("Level 1: 真实鼠标点击播放候选元素");
        }
        hook_manager.trigger_play(play_js.as_deref()).await.ok();
        tokio::time::sleep(Duration::from_millis(500)).await;

        // 12. 主事件循环
        diag.log_phase("进入主事件循环...");
        let play_dur = self.play_duration;
        let max_wait = Duration::from_secs(play_dur.as_secs() + 30);
        let start = std::time::Instant::now();
        let mut second_click_triggered = clicked;
        let mut third_triggered = false;
        let mut poll_interval = tokio::time::interval(Duration::from_secs(1));
        let mut last_video_state = serde_json::json!({});

        loop {
            tokio::select! {
                event = event_rx.recv() => {
                    let Some(event) = event else { break };
                    diag.log_event(&event);
                    collector.on_event(&event);
                }
                _ = poll_interval.tick() => {
                    if let Ok(state) = hook_manager.poll_video_state().await {
                        last_video_state = state.clone();
                        let ct = state.get("ct").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let vw = state.get("vw").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                        let vh = state.get("vh").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                        let vdur = state.get("vdur").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let paused = state.get("paused").and_then(|v| v.as_bool()).unwrap_or(true);
                        let ready_state = state.get("readyState").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                        let decoded = state.get("webkitDecoded").and_then(|v| v.as_u64()).unwrap_or(0);
                        let dropped = state.get("webkitDropped").and_then(|v| v.as_u64()).unwrap_or(0);
                        let ended = state.get("ended").and_then(|v| v.as_bool()).unwrap_or(false);

                        collector.update_video_state(ct, paused, ready_state, vw, vh, vdur, decoded, dropped);

                        if ended {
                            let _ = event_tx.send(VideoEvent::PlayEnded {
                                player_id: None,
                                meta: EventMeta::now(),
                            });
                            break;
                        }
                    }

                    let elapsed = start.elapsed();

                    // 多级播放触发
                    if !collector.metrics.play_success && !second_click_triggered && elapsed >= Duration::from_secs(5) {
                        diag.log_phase("Level 2: 再次真实点击播放候选元素");
                        if !hook_manager.click_play_candidate(&play_selectors).await.unwrap_or(false) {
                            hook_manager.click_center().await.ok();
                        }
                        second_click_triggered = true;
                    }
                    if !collector.metrics.play_success && !third_triggered && elapsed >= Duration::from_secs(15) {
                        diag.log_phase("Level 3: 再次尝试 JS 触发");
                        hook_manager.trigger_play(None).await.ok();
                        third_triggered = true;
                    }

                    if elapsed >= max_wait {
                        diag.log_phase("主循环超时退出");
                        break;
                    }

                    if collector.metrics.play_success && collector.has_played_for(play_dur) {
                        diag.log_phase("播放时间已够，退出主循环");
                        break;
                    }
                }
            }
        }

        // 13. 获取截图和标题
        let page_title = hook_manager.page_title().await.ok();
        collector.set_page_title(page_title.unwrap_or_default());

        if !collector.metrics.play_success && collector.metrics.error.is_none() {
            let page_text = hook_manager
                .playback_diagnostic_text()
                .await
                .unwrap_or_default();
            let error = classify_playback_failure(
                video_count,
                &last_video_state,
                collector.metrics.total_bytes,
                &page_text,
            );
            collector.set_error(error);
        }

        let screenshot = hook_manager.screenshot().await.ok();
        if let Some(ref data) = screenshot {
            collector.set_screenshot(data.clone());
        }

        // 14. 最终化
        let metrics = collector.finalize();
        let _ = event_tx.send(VideoEvent::MetricsFinalized {
            play_detected: metrics.play_success,
            first_play_time_ms: metrics.first_play_time_ms.unwrap_or(0.0),
            total_buffer_count: metrics.buffer_count,
            total_buffer_time_ms: metrics.buffer_time_ms,
            meta: EventMeta::now(),
        });

        diag.log_phase(&format!("测试完成: play_success={}", metrics.play_success));
        let result = VideoTestResult::from(metrics);
        chrome.shutdown().await;
        info!(
            "释放浏览器槽位: type=video url={} held_ms={:.0}",
            url,
            browser_started.elapsed().as_secs_f64() * 1000.0
        );
        drop(permit);
        result
    }
}

fn classify_playback_failure(
    video_count: u32,
    state: &serde_json::Value,
    total_bytes: u64,
    page_text: &str,
) -> String {
    let paused = state
        .get("paused")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let ready_state = state
        .get("readyState")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let network_state = state
        .get("networkState")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let current_time = state.get("ct").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let lower_text = page_text.to_lowercase();

    if [
        "captcha",
        "人机验证",
        "安全验证",
        "robot check",
        "verify you are human",
    ]
    .iter()
    .any(|keyword| lower_text.contains(keyword))
    {
        return "captcha_or_bot_check: 页面要求完成验证码或人机验证".into();
    }
    if [
        "login required",
        "sign in to confirm",
        "sign in to watch",
        "log in to watch",
        "请先登录后观看",
        "登录后观看",
        "请先登录",
        "会员登录",
    ]
    .iter()
    .any(|keyword| lower_text.contains(keyword))
    {
        return "login_required: 页面要求登录后才能播放".into();
    }
    if [
        "not available in your country",
        "区域限制",
        "地区限制",
        "版权限制",
        "unavailable in your region",
    ]
    .iter()
    .any(|keyword| lower_text.contains(keyword))
    {
        return "geo_or_rights_blocked: 视频受地区、版权或访问范围限制".into();
    }

    if video_count == 0 {
        return "no_video_element: 页面未发现 HTML5 video 元素".into();
    }

    if total_bytes == 0 && ready_state < 2 {
        return format!(
            "no_media_network: 发现 video 元素但没有媒体流量或可播放数据 (readyState={ready_state}, networkState={network_state})"
        );
    }

    if paused && current_time <= 0.0 {
        return "autoplay_blocked: video 元素存在但播放未开始，可能需要用户手势、登录、验证码或地区授权".into();
    }

    format!(
        "unknown_playback_failure: video 元素存在但播放时间未持续推进 (currentTime={current_time:.2}, readyState={ready_state}, bytes={total_bytes})"
    )
}
