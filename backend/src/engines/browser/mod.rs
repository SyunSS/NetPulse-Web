pub mod provider;

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::engines::browser::provider::{BrowserPage, BrowserProvider};

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

/// 浏览器引擎 — 通过 BrowserProvider trait 驱动浏览器
pub struct BrowserEngine {
    provider: Arc<Box<dyn BrowserProvider>>,
    chrome_path: String,
    headless: bool,
    timeout: Duration,
}

impl BrowserEngine {
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
        }
    }

    /// 测试页面，采集 Performance 指标和截图
    pub async fn test_page(&self, url: &str) -> BrowserResult {
        info!("浏览器测试开始: {}", url);
        let total_start = std::time::Instant::now();

        // 启动浏览器
        let handle = match self.provider.launch(self.chrome_path.clone(), self.headless, vec![]).await {
            Ok(h) => h,
            Err(e) => {
                error!("浏览器启动失败: {}", e);
                return err_result(&e.to_string());
            }
        };

        // 创建页面
        let page = match handle.new_page().await {
            Ok(p) => p,
            Err(e) => {
                error!("创建页面失败: {}", e);
                return err_result(&e.to_string());
            }
        };

        // 导航
        if let Err(e) = page.navigate_to(url).await {
            error!("导航失败: {}", e);
            return err_result(&e.to_string());
        }

        if let Err(e) = page.wait_for_load().await {
            debug!("等待导航完成: {}", e);
        }

        let nav_elapsed = total_start.elapsed().as_secs_f64() * 1000.0;

        // 等待页面充分渲染
        tokio::time::sleep(Duration::from_secs(3)).await;

        // 注入 JS 采集 Performance API 数据
        let perf_data: PerfData = match page.evaluate_sync(PERF_JS) {
            Ok(val) => serde_json::from_value(val).unwrap_or_default(),
            Err(e) => {
                debug!("Performance API 第一次采集失败: {}", e);
                tokio::time::sleep(Duration::from_millis(500)).await;
                page.evaluate_sync(PERF_JS)
                    .ok()
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default()
            }
        };

        // 获取页面标题
        let title = page.evaluate_sync("document.title")
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        // 获取最终 URL
        let final_url = page.evaluate_sync("window.location.href")
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        // 截图
        let screenshot = page.screenshot().ok();

        debug!(
            "浏览器测试完成: {} - FP:{:?} FCP:{:?} DCL:{:?} Load:{:?}",
            url, perf_data.first_paint, perf_data.first_contentful_paint,
            perf_data.dom_content_loaded, perf_data.load_event_end
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
}

fn err_result(msg: &str) -> BrowserResult {
    BrowserResult {
        fp_ms: None, fcp_ms: None, dom_content_loaded_ms: None, load_event_ms: None,
        page_open_time_ms: None, first_paint_ms: None,
        resource_count: None, resource_total_size: None,
        final_url: None, page_title: None, screenshot: None,
        error: Some(msg.to_string()),
    }
}

// ─── Performance JS ──────────────────────────────────

#[derive(Debug, Default, Deserialize)]
struct PerfData {
    #[serde(rename = "first_paint", default)]
    first_paint: Option<f64>,
    #[serde(rename = "first_contentful_paint", default)]
    first_contentful_paint: Option<f64>,
    #[serde(rename = "dom_content_loaded", default)]
    dom_content_loaded: Option<f64>,
    #[serde(rename = "load_event_end", default)]
    load_event_end: Option<f64>,
    #[serde(rename = "resource_count", default)]
    resource_count: i32,
    #[serde(rename = "resource_total_size", default)]
    resource_total_size: i32,
}

const PERF_JS: &str = r#"
(function() {
    var t = performance.timing;
    var domContentLoaded = (t.domContentLoadedEventEnd > 0) ? (t.domContentLoadedEventEnd - t.navigationStart) : null;
    var loadEventEnd = (t.loadEventEnd > 0) ? (t.loadEventEnd - t.navigationStart) : null;
    var firstPaint = null, firstContentfulPaint = null;
    var paintEntries = performance.getEntriesByType('paint');
    for (var i=0; i<paintEntries.length; i++) {
        var e = paintEntries[i];
        if (e.name === 'first-paint') firstPaint = e.startTime;
        if (e.name === 'first-contentful-paint') firstContentfulPaint = e.startTime;
    }
    var resources = performance.getEntriesByType('resource');
    var totalSize = 0;
    for (var i=0; i<resources.length; i++) {
        totalSize += (resources[i].transferSize || resources[i].encodedBodySize || 0);
    }
    return {
        first_paint: firstPaint,
        first_contentful_paint: firstContentfulPaint,
        dom_content_loaded: domContentLoaded,
        load_event_end: loadEventEnd,
        resource_count: resources.length,
        resource_total_size: totalSize
    };
})()
"#;
