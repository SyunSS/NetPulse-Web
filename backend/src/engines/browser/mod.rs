use std::time::Duration;

use headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption;
use headless_chrome::{Browser, LaunchOptions};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

/// 浏览器测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserResult {
    pub fp_ms: Option<f64>,
    pub fcp_ms: Option<f64>,
    pub dom_content_loaded_ms: Option<f64>,
    pub load_event_ms: Option<f64>,
    pub page_open_time_ms: Option<f64>,
    pub first_paint_ms: Option<f64>,
    pub resource_count: Option<i32>,
    pub resource_total_size: Option<i32>,
    pub final_url: Option<String>,
    pub page_title: Option<String>,
    pub screenshot: Option<Vec<u8>>,
    pub error: Option<String>,
}

/// 浏览器引擎 — 基于 headless_chrome (CDP)
pub struct BrowserEngine {
    chrome_path: String,
    headless: bool,
    timeout: Duration,
}

impl BrowserEngine {
    pub fn new(chrome_path: &str, headless: bool, timeout: Duration) -> Self {
        Self {
            chrome_path: chrome_path.to_string(),
            headless,
            timeout,
        }
    }

    /// 测试页面，采集 Performance 指标和截图
    pub async fn test_page(&self, url: &str) -> BrowserResult {
        info!("浏览器测试开始: {}", url);

        let chrome_path = self.chrome_path.clone();
        let headless = self.headless;
        let timeout = self.timeout;
        let url = url.to_string();

        // headless_chrome 是同步的，用 spawn_blocking 包装
        let result = tokio::task::spawn_blocking(move || {
            // 设置超时
            let result = std::thread::spawn(move || test_page_blocking(&chrome_path, headless, &url, timeout))
                .join();

            match result {
                Ok(r) => r,
                Err(_) => BrowserResult {
                    fp_ms: None,
                    fcp_ms: None,
                    dom_content_loaded_ms: None,
                    load_event_ms: None,
                    page_open_time_ms: None,
                    first_paint_ms: None,
                    resource_count: None,
                    resource_total_size: None,
                    final_url: None,
                    page_title: None,
                    screenshot: None,
                    error: Some("浏览器线程异常退出".to_string()),
                },
            }
        })
        .await;

        match result {
            Ok(r) => r,
            Err(e) => {
                error!("spawn_blocking 失败: {}", e);
                BrowserResult {
                    fp_ms: None,
                    fcp_ms: None,
                    dom_content_loaded_ms: None,
                    load_event_ms: None,
                    page_open_time_ms: None,
                    first_paint_ms: None,
                    resource_count: None,
                    resource_total_size: None,
                    final_url: None,
                    page_title: None,
                    screenshot: None,
                    error: Some(format!("任务执行失败: {}", e)),
                }
            }
        }
    }
}

/// 同步阻塞的页面测试
fn test_page_blocking(chrome_path: &str, headless: bool, url: &str, _timeout: Duration) -> BrowserResult {
    // 构建 LaunchOptions
    let path = std::path::PathBuf::from(chrome_path);
    let options = match LaunchOptions::default_builder()
        .headless(headless)
        .sandbox(false)
        .enable_gpu(false)
        .enable_logging(false)
        .window_size(Some((1920, 1080)))
        .path(Some(path))
        .build()
    {
        Ok(o) => o,
        Err(e) => {
            error!("LaunchOptions 构建失败: {}", e);
            return BrowserResult {
                fp_ms: None,
                fcp_ms: None,
                dom_content_loaded_ms: None,
                load_event_ms: None,
                page_open_time_ms: None,
                first_paint_ms: None,
                resource_count: None,
                resource_total_size: None,
                final_url: None,
                page_title: None,
                screenshot: None,
                error: Some(format!("浏览器启动配置失败: {}", e)),
            };
        }
    };

    let browser = match Browser::new(options) {
        Ok(b) => b,
        Err(e) => {
            error!("浏览器启动失败: {}", e);
            return BrowserResult {
                fp_ms: None,
                fcp_ms: None,
                dom_content_loaded_ms: None,
                load_event_ms: None,
                page_open_time_ms: None,
                first_paint_ms: None,
                resource_count: None,
                resource_total_size: None,
                final_url: None,
                page_title: None,
                screenshot: None,
                error: Some(format!("浏览器启动失败: {}", e)),
            };
        }
    };

    let tab = match browser.new_tab() {
        Ok(t) => t,
        Err(e) => {
            error!("创建标签页失败: {}", e);
            return error_result(&format!("创建标签页失败: {}", e));
        }
    };

    // 导航到目标 URL
    let nav_start = std::time::Instant::now();
    if let Err(e) = tab.navigate_to(url) {
        error!("导航失败: {} - {}", url, e);
        return error_result(&format!("导航失败: {}", e));
    }

    if let Err(e) = tab.wait_until_navigated() {
        error!("等待导航完成失败: {}", e);
        // 继续执行，尝试采集已有数据
    }

    let nav_elapsed = nav_start.elapsed().as_secs_f64() * 1000.0;

    // PERF_JS 内部等待 readyState + 8s 超时，这里再等 1s 兜底
    std::thread::sleep(Duration::from_secs(1));

    // 注入 JS 采集 Performance API 数据
    let perf_data: PerfData = match tab.evaluate(PERF_JS, true) {
        Ok(remote_obj) => {
            match &remote_obj.value {
                Some(val) => serde_json::from_value(val.clone()).unwrap_or_default(),
                None => PerfData::default(),
            }
        }
        Err(e) => {
            debug!("Performance API 采集失败: {}", e);
            PerfData::default()
        }
    };

    // 采集页面标题
    let title = tab
        .evaluate("document.title", false)
        .ok()
        .and_then(|r| r.value)
        .and_then(|v| v.as_str().map(|s| s.to_string()));

    // 采集最终 URL
    let final_url = tab
        .evaluate("window.location.href", false)
        .ok()
        .and_then(|r| r.value)
        .and_then(|v| v.as_str().map(|s| s.to_string()));

    // 截图
    let screenshot = tab
        .capture_screenshot(CaptureScreenshotFormatOption::Png, None, None, true)
        .ok();

    debug!(
        "浏览器测试完成: {} - FP:{:?} FCP:{:?} 资源数:{}",
        url, perf_data.first_paint, perf_data.first_contentful_paint, perf_data.resource_count
    );

    BrowserResult {
        fp_ms: perf_data.first_paint,
        fcp_ms: perf_data.first_contentful_paint,
        dom_content_loaded_ms: perf_data.dom_content_loaded,
        load_event_ms: perf_data.load_event_end,
        page_open_time_ms: Some(nav_elapsed),
        first_paint_ms: perf_data.first_paint,
        resource_count: Some(perf_data.resource_count),
        resource_total_size: Some(perf_data.resource_total_size),
        final_url,
        page_title: title,
        screenshot,
        error: None,
    }
}

/// 生成错误结果
fn error_result(msg: &str) -> BrowserResult {
    BrowserResult {
        fp_ms: None,
        fcp_ms: None,
        dom_content_loaded_ms: None,
        load_event_ms: None,
        page_open_time_ms: None,
        first_paint_ms: None,
        resource_count: None,
        resource_total_size: None,
        final_url: None,
        page_title: None,
        screenshot: None,
        error: Some(msg.to_string()),
    }
}

/// Performance API 采集的数据
#[derive(Debug, Default, Deserialize)]
struct PerfData {
    #[serde(rename = "firstPaint", default)]
    first_paint: Option<f64>,
    #[serde(rename = "firstContentfulPaint", default)]
    first_contentful_paint: Option<f64>,
    #[serde(rename = "domContentLoaded", default)]
    dom_content_loaded: Option<f64>,
    #[serde(rename = "loadEventEnd", default)]
    load_event_end: Option<f64>,
    #[serde(rename = "resourceCount", default)]
    resource_count: i32,
    #[serde(rename = "resourceTotalSize", default)]
    resource_total_size: i32,
}

/// 注入的 JS 脚本，采集 Performance API 数据
const PERF_JS: &str = r#"
(async () => {
    // 等待页面完全加载（最多 8 秒）
    const waitForLoad = () => new Promise(resolve => {
        if (document.readyState === 'complete') return resolve();
        const timer = setTimeout(() => resolve(), 8000);
        window.addEventListener('load', () => { clearTimeout(timer); resolve(); }, { once: true });
    });
    await waitForLoad();

    // 再等 1.5 秒让所有 XHR/fetch 完成
    await new Promise(r => setTimeout(r, 1500));

    // 用 PerformanceObserver 收集所有资源条目（包括 lazy-loaded）
    const allEntries = performance.getEntriesByType('resource');
    let resourceTotalSize = 0;
    let videoSize = 0;
    let countWithSize = 0;
    for (const entry of allEntries) {
        const size = entry.transferSize || entry.encodedBodySize || entry.decodedBodySize || 0;
        resourceTotalSize += size;
        // 视频/HLS 资源单独统计
        if (entry.initiatorType === 'video' || entry.name.match(/\.(mp4|m3u8|ts|m4s|webm|flv)/)) {
            videoSize += size;
        }
        if (size > 0) countWithSize++;
    }

    // 加上 navigation entry 自身
    const navEntries = performance.getEntriesByType('navigation');
    let navSize = 0, firstPaint = null, firstContentfulPaint = null, domContentLoaded = null, loadEventEnd = null;
    if (navEntries.length > 0) {
        const nav = navEntries[0];
        navSize = nav.transferSize || nav.encodedBodySize || 0;
        domContentLoaded = nav.domContentLoadedEventEnd;
        loadEventEnd = nav.loadEventEnd;
    }

    // Paint Timing
    const paintEntries = performance.getEntriesByType('paint');
    for (const entry of paintEntries) {
        if (entry.name === 'first-paint' && firstPaint === null) firstPaint = entry.startTime;
        if (entry.name === 'first-contentful-paint' && firstContentfulPaint === null) firstContentfulPaint = entry.startTime;
    }

    return {
        firstPaint: firstPaint,
        firstContentfulPaint: firstContentfulPaint,
        domContentLoaded: domContentLoaded,
        loadEventEnd: loadEventEnd,
        resourceCount: allEntries.length,
        resourceTotalSize: resourceTotalSize + navSize,
        videoSize: videoSize,
        countWithSize: countWithSize
    };
})()
"#;
