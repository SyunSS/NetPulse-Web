use std::ffi::OsStr;
use std::time::Duration;

use headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption;
use headless_chrome::{Browser, LaunchOptions};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::config::VideoPlatformConfig;

/// 视频测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTestResult {
    pub platform: String,
    pub first_play_time_ms: Option<f64>,
    pub buffer_count: Option<i32>,
    pub total_buffer_time_ms: Option<f64>,
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

/// 视频测试引擎 — 配置驱动
pub struct VideoEngine {
    chrome_path: String,
    headless: bool,
    timeout: Duration,
    play_duration: Duration,
}

impl VideoEngine {
    pub fn new(chrome_path: &str, headless: bool, timeout: Duration) -> Self {
        Self {
            chrome_path: chrome_path.to_string(),
            headless,
            timeout,
            play_duration: Duration::from_secs(15),
        }
    }

    /// 测试视频页面（需传入匹配后的平台配置）
    pub async fn test_page(&self, url: &str, platform_cfg: &VideoPlatformConfig) -> VideoTestResult {
        info!("视频测试开始: {} (平台: {})", url, platform_cfg.name);

        // 仅检测可访问性的平台（如 Netflix）
        if platform_cfg.is_detect_only() {
            return self.test_detect_only(url, platform_cfg).await;
        }

        let chrome_path = self.chrome_path.clone();
        let headless = self.headless;
        let play_duration = self.play_duration;
        let url = url.to_string();
        let platform_name = platform_cfg.name.clone();
        let wait_seconds = platform_cfg.wait_seconds.unwrap_or(1);
        let video_selector = platform_cfg.video_selector.clone().unwrap_or_else(|| "video".to_string());

        let result = tokio::task::spawn_blocking(move || {
            test_video_blocking(
                &chrome_path,
                headless,
                &url,
                &platform_name,
                &video_selector,
                wait_seconds,
                play_duration,
            )
        })
        .await;

        match result {
            Ok(r) => r,
            Err(e) => {
                error!("视频测试 spawn_blocking 失败: {}", e);
                VideoTestResult {
                    platform: platform_cfg.name.clone(),
                    first_play_time_ms: None,
                    buffer_count: None,
                    total_buffer_time_ms: None,
                    play_success: false,
                    video_download_speed: None,
                    video_size: None,
                    video_duration_ms: None,
                    dropped_frames: None,
                    decoded_frames: None,
                    page_title: None,
                    screenshot: None,
                    error: Some(format!("任务执行失败: {}", e)),
                }
            }
        }
    }

    /// 仅检测可访问性（Netflix 等 DRM 平台）
    async fn test_detect_only(&self, url: &str, platform_cfg: &VideoPlatformConfig) -> VideoTestResult {
        let chrome_path = self.chrome_path.clone();
        let headless = self.headless;
        let url = url.to_string();
        let platform_name = platform_cfg.name.clone();

        let result = tokio::task::spawn_blocking(move || {
            let path = std::path::PathBuf::from(&chrome_path);
            let options = LaunchOptions::default_builder()
                .headless(headless)
                .sandbox(false)
                .enable_gpu(false)
                .enable_logging(false)
                .window_size(Some((1920, 1080)))
                .path(Some(path))
                .build();

            let options = match options {
                Ok(o) => o,
                Err(e) => return error_result(&platform_name, &format!("浏览器配置失败: {}", e)),
            };

            let browser = match Browser::new(options) {
                Ok(b) => b,
                Err(e) => return error_result(&platform_name, &format!("浏览器启动失败: {}", e)),
            };

            let tab = match browser.new_tab() {
                Ok(t) => t,
                Err(e) => return error_result(&platform_name, &format!("创建标签页失败: {}", e)),
            };

            let accessible = tab.navigate_to(&url).is_ok()
                && tab.wait_until_navigated().is_ok();

            let title = tab
                .evaluate("document.title", false)
                .ok()
                .and_then(|r| r.value)
                .and_then(|v| v.as_str().map(|s| s.to_string()));

            VideoTestResult {
                platform: platform_name,
                first_play_time_ms: None,
                buffer_count: None,
                total_buffer_time_ms: None,
                play_success: accessible,
                video_download_speed: None,
                video_size: None,
                video_duration_ms: None,
                dropped_frames: None,
                decoded_frames: None,
                page_title: title,
                screenshot: None,
                error: if accessible {
                    None
                } else {
                    Some("页面无法访问".to_string())
                },
            }
        })
        .await;

        match result {
            Ok(r) => r,
            Err(e) => error_result(&platform_cfg.name, &format!("任务失败: {}", e)),
        }
    }
}

/// 同步阻塞的视频测试
fn test_video_blocking(
    chrome_path: &str,
    headless: bool,
    url: &str,
    platform_name: &str,
    video_selector: &str,
    wait_seconds: u64,
    play_duration: Duration,
) -> VideoTestResult {
    let path = std::path::PathBuf::from(chrome_path);
    let autoplay_arg: &OsStr = OsStr::new("--autoplay-policy=no-user-gesture-required");
    let mute_arg: &OsStr = OsStr::new("--mute-audio");
    let options = match LaunchOptions::default_builder()
        .headless(headless)
        .sandbox(false)
        .enable_gpu(false)
        .enable_logging(false)
        .window_size(Some((1920, 1080)))
        .path(Some(path))
        .args(vec![autoplay_arg, mute_arg])
        .build()
    {
        Ok(o) => o,
        Err(e) => return error_result(platform_name, &format!("浏览器配置失败: {}", e)),
    };

    let browser = match Browser::new(options) {
        Ok(b) => b,
        Err(e) => return error_result(platform_name, &format!("浏览器启动失败: {}", e)),
    };

    let tab = match browser.new_tab() {
        Ok(t) => t,
        Err(e) => return error_result(platform_name, &format!("创建标签页失败: {}", e)),
    };

    if let Err(e) = tab.navigate_to(url) {
        return error_result(platform_name, &format!("导航失败: {}", e));
    }
    if let Err(e) = tab.wait_until_navigated() {
        debug!("等待导航完成: {}", e);
    }

    // 等待页面加载（配置的等待秒数）
    std::thread::sleep(Duration::from_secs(wait_seconds));

    // 注入视频监听脚本 — 使用配置的 CSS 选择器定位 video 元素
    let inject_result = tab.evaluate(&build_inject_js(video_selector, play_duration), true);
    let video_data: VideoJsData = match inject_result {
        Ok(remote_obj) => match &remote_obj.value {
            Some(val) => serde_json::from_value(val.clone()).unwrap_or_default(),
            None => VideoJsData::default(),
        },
        Err(e) => {
            debug!("视频数据采集失败: {}", e);
            VideoJsData::default()
        }
    };

    // 等待播放采集
    std::thread::sleep(play_duration);

    // 采集最终结果
    let final_data: VideoJsData = match tab.evaluate(&build_collect_js(video_selector), true) {
        Ok(remote_obj) => match &remote_obj.value {
            Some(val) => serde_json::from_value(val.clone()).unwrap_or_default(),
            None => VideoJsData::default(),
        },
        Err(e) => {
            debug!("最终数据采集失败: {}", e);
            VideoJsData::default()
        }
    };

    let title = tab
        .evaluate("document.title", false)
        .ok()
        .and_then(|r| r.value)
        .and_then(|v| v.as_str().map(|s| s.to_string()));

    let screenshot = tab
        .capture_screenshot(CaptureScreenshotFormatOption::Png, None, None, true)
        .ok();

    debug!(
        "视频测试完成: {} (平台:{}) - 播放成功:{} 缓冲:{}次",
        url, platform_name, final_data.play_success, final_data.buffer_count
    );

    VideoTestResult {
        platform: platform_name.to_string(),
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
        } else {
            None
        },
    }
}

/// 构建注入 JS — 使用配置的 CSS 选择器 + 多重 fallback
fn build_inject_js(video_selector: &str, play_duration: Duration) -> String {
    let play_ms = play_duration.as_millis();
    let selector_escaped = video_selector.replace('\\', "\\\\").replace('\'', "\\'");
    format!(
        r#"
(() => {{
    // 多选择器 fallback：先试配置的，找不到再试通用 video
    const selectors = ['{0}', 'video', '.video video', '#video', 'video.html5-main-video'];
    let video = null;
    for (const sel of selectors) {{
        const el = document.querySelector(sel);
        if (el && el.tagName === 'VIDEO') {{ video = el; break; }}
    }}
    if (!video) {{
        // 找页面里所有 video 元素
        const all = document.querySelectorAll('video');
        if (all.length > 0) video = all[0];
    }}
    if (!video) {{
        window.__videoStats = {{ page_loaded: true, error: 'no_video_element', play_success: false }};
        return {{ page_loaded: true, error: 'no_video_element' }};
    }}

    window.__videoStats = {{
        first_play_time_ms: null,
        buffer_count: 0,
        total_buffer_time_ms: 0,
        play_success: false,
        video_download_speed: null,
        video_size: null,
        video_duration_ms: null,
        dropped_frames: 0,
        decoded_frames: 0,
        page_loaded: true,
        _buffer_start: null,
        _start_time: performance.now()
    }};

    // 监听 playing 事件
    video.addEventListener('playing', () => {{
        if (window.__videoStats.first_play_time_ms === null) {{
            window.__videoStats.first_play_time_ms = performance.now() - window.__videoStats._start_time;
            window.__videoStats.play_success = true;
        }}
        if (window.__videoStats._buffer_start !== null) {{
            window.__videoStats.total_buffer_time_ms += performance.now() - window.__videoStats._buffer_start;
            window.__videoStats._buffer_start = null;
        }}
    }});

    video.addEventListener('waiting', () => {{
        window.__videoStats.buffer_count++;
        window.__videoStats._buffer_start = performance.now();
    }});

    video.addEventListener('loadedmetadata', () => {{
        window.__videoStats.video_duration_ms = video.duration ? video.duration * 1000 : null;
    }});

    // 延迟检测：如果 5 秒内已经播放过，标记为成功
    setTimeout(() => {{
        if (!window.__videoStats.play_success && !video.paused && video.currentTime > 0) {{
            window.__videoStats.first_play_time_ms = performance.now() - window.__videoStats._start_time;
            window.__videoStats.play_success = true;
        }}
    }}, 5000);

    // 尝试播放
    video.play().catch(() => {{}});

    // 采集资源大小
    const resources = performance.getEntriesByType('resource');
    let videoSize = 0;
    for (const r of resources) {{
        if (r.initiatorType === 'video' || r.initiatorType === 'xmlhttprequest' ||
            r.name.includes('.mp4') || r.name.includes('.m3u8') || r.name.includes('.mpd') ||
            r.name.includes('.flv')) {{
            videoSize += r.transferSize || 0;
        }}
    }}
    window.__videoStats.video_size = videoSize || null;

    return {{ injected: true, video_tag: video.tagName }};
}})()
"#, selector_escaped
    )
}

/// 采集最终数据的 JS — 使用配置的 CSS 选择器
fn build_collect_js(video_selector: &str) -> String {
    let selector_escaped = video_selector.replace('\\', "\\\\").replace('\'', "\\'");
    format!(
        r#"
(() => {{
    if (!window.__videoStats) return {{ play_success: false }};
    const s = window.__videoStats;

    let video = document.querySelector('{0}');
    if (!video) video = document.querySelector('video');

    if (video) {{
        if (video.webkitDecodedFrameCount !== undefined) s.decoded_frames = video.webkitDecodedFrameCount;
        if (video.webkitDroppedFrameCount !== undefined) s.dropped_frames = video.webkitDroppedFrameCount;
        if (video.duration && !s.video_duration_ms) s.video_duration_ms = video.duration * 1000;
    }}

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
"#, selector_escaped
    )
}

/// JS 采集的数据
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

fn error_result(platform: &str, msg: &str) -> VideoTestResult {
    VideoTestResult {
        platform: platform.to_string(),
        first_play_time_ms: None,
        buffer_count: None,
        total_buffer_time_ms: None,
        play_success: false,
        video_download_speed: None,
        video_size: None,
        video_duration_ms: None,
        dropped_frames: None,
        decoded_frames: None,
        page_title: None,
        screenshot: None,
        error: Some(msg.to_string()),
    }
}
