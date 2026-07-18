pub mod collectors;
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
    // 新增 CDP 采集字段
    pub html_size: Option<i32>,
    pub css_size: Option<i32>,
    pub js_size: Option<i32>,
    pub image_size: Option<i32>,
    pub font_size: Option<i32>,
    pub total_requests: Option<i32>,
    pub failed_requests: Option<i32>,
    pub lcp_ms: Option<f64>,
    pub cls: Option<f64>,
    pub tti_ms: Option<f64>,
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
            Err(e) => { error!("浏览器启动失败: {}", e); return err_result(&e.to_string()); }
        };
        let page = match handle.new_page().await {
            Ok(p) => p,
            Err(e) => { error!("创建页面失败: {}", e); return err_result(&e.to_string()); }
        };

        // 启用 CDP 域
        let _ = page.send_cdp("Network.enable", serde_json::json!({}));
        let _ = page.send_cdp("Media.enable", serde_json::json!({}));

        // 页面采集器
        let page_collector = std::sync::Arc::new(collectors::PageCollector::new());

        // 导航
        page_collector.record_navigation();
        if let Err(e) = page.navigate_to(url).await {
            error!("导航失败: {}", e); return err_result(&e.to_string());
        }
        if let Err(e) = page.wait_for_load().await {
            debug!("等待导航完成: {}", e);
        }

        let nav_elapsed = total_start.elapsed().as_secs_f64() * 1000.0;

        // 等待渲染 + 资源加载
        tokio::time::sleep(Duration::from_secs(3)).await;

        // 记录 Load 时间
        page_collector.record_load();

        // 通过 JS 获取 Performance Paint Timing (CDP Page 域备选)
        let perf_js = r#"JSON.stringify((function(){
            var p = performance.getEntriesByType('paint');
            var fp = null, fcp = null;
            for (var i=0; i<p.length; i++) {
                if (p[i].name === 'first-paint') fp = p[i].startTime;
                if (p[i].name === 'first-contentful-paint') fcp = p[i].startTime;
            }
            return {fp: fp, fcp: fcp};
        })())"#;
        let paint: PaintData = page.evaluate_sync(perf_js)
            .ok().and_then(|v| v.as_str().and_then(|s| serde_json::from_str(s).ok()))
            .unwrap_or_default();

        // 通过 JS 获取资源统计
        let net_js = collectors::NetworkCollector::collect_js();
        let net_data: serde_json::Value = page.evaluate_sync(net_js)
            .ok().and_then(|v| v.as_str().and_then(|s| serde_json::from_str(s).ok()))
            .unwrap_or(serde_json::Value::Null);
        let net_metrics = collectors::NetworkCollector::parse(net_data);

        // 页面标题 + 最终 URL
        let title = page.evaluate_sync("document.title").ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let final_url = page.evaluate_sync("window.location.href").ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        // 截图
        let screenshot = page.screenshot().ok();

        let pg = page_collector.snapshot();

        BrowserResult {
            fp_ms: paint.fp,
            fcp_ms: paint.fcp,
            dom_content_loaded_ms: pg.dom_content_loaded_ms,
            load_event_ms: pg.load_event_ms,
            page_open_time_ms: Some(nav_elapsed),
            first_paint_ms: paint.fp,
            resource_count: Some(net_metrics.request_count),
            resource_total_size: Some(net_metrics.total_transfer_size as i32),
            final_url,
            page_title: title,
            screenshot,
            error: None,
            // 新增字段
            html_size: Some(net_metrics.html_size as i32),
            css_size: Some(net_metrics.css_size as i32),
            js_size: Some(net_metrics.js_size as i32),
            image_size: Some(net_metrics.image_size as i32),
            font_size: Some(net_metrics.font_size as i32),
            total_requests: Some(net_metrics.request_count),
            failed_requests: Some(net_metrics.failed_count),
            lcp_ms: None,  // LCP 需要 Tracing 域, 暂不采集
            cls: None,
            tti_ms: None,
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
        html_size: None, css_size: None, js_size: None, image_size: None, font_size: None,
        total_requests: None, failed_requests: None,
        lcp_ms: None, cls: None, tti_ms: None,
    }
}

#[derive(Debug, Default, Deserialize)]
struct PaintData {
    #[serde(default)] fp: Option<f64>,
    #[serde(default)] fcp: Option<f64>,
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

const PERF_JS: &str = r#"JSON.stringify((function() {
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
})())"#;
