pub mod collectors;
pub mod provider;

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::config::BrowserConfig;

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
    pub nav_dns_ms: Option<f64>,
    pub nav_connect_ms: Option<f64>,
    pub nav_ttfb_ms: Option<f64>,
    pub site_size_kb: Option<f64>,
    pub avg_speed_kbps: Option<f64>,
    pub total_speed_kbps: Option<f64>,
    pub first_screen_ratio: Option<f64>,
}

/// 浏览器引擎 — 直接使用 headless_chrome
pub struct BrowserEngine {
    chrome_config: BrowserConfig,
    timeout: Duration,
}

impl BrowserEngine {
    pub fn new(chrome_config: BrowserConfig, timeout: Duration) -> Self {
        Self { chrome_config, timeout }
    }

    /// 测试页面，采集 Performance 指标和截图
    pub async fn test_page(&self, url: &str) -> BrowserResult {
        info!("浏览器测试开始: {}", url);
        let total_start = std::time::Instant::now();

        let browser = match provider::launch_browser(&self.chrome_config) {
            Ok(b) => b,
            Err(e) => { error!("浏览器启动失败: {}", e); return err_result(&e.to_string()); }
        };
        let page = match provider::new_page(&browser) {
            Ok(p) => p,
            Err(e) => { error!("创建页面失败: {}", e); return err_result(&e.to_string()); }
        };

        let _ = page.send_cdp("Network.enable", serde_json::json!({}));

        let page_collector = Arc::new(collectors::PageCollector::new());

        page_collector.record_navigation();
        if let Err(e) = page.navigate_to(url) {
            error!("导航失败: {}", e); return err_result(&e.to_string());
        }
        if let Err(e) = page.wait_for_load() {
            debug!("等待导航完成: {}", e);
        }

        let lcp_js = collectors::NetworkCollector::lcp_inject_js();
        let _ = page.evaluate_sync(lcp_js);

        let nav_elapsed = total_start.elapsed().as_secs_f64() * 1000.0;

        tokio::time::sleep(Duration::from_secs(5)).await;

        page_collector.record_load();

        let perf_js = collectors::NetworkCollector::collect_js();
        let perf_data: serde_json::Value = page.evaluate_sync(perf_js)
            .ok().and_then(|v| v.as_str().and_then(|s| serde_json::from_str(s).ok()))
            .unwrap_or(serde_json::Value::Null);
        let net_metrics = collectors::NetworkCollector::parse(perf_data.clone());

        let title = page.evaluate_sync("document.title").ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        let final_url = page.evaluate_sync("window.location.href").ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        let screenshot = page.screenshot().ok();

        let pg = page_collector.snapshot();

        let nav_dns = net_metrics.dns_ms.or_else(|| {
            perf_data.get("dns").and_then(|v| v.as_f64()).filter(|&x| x > 0.0)
        });
        let nav_connect = net_metrics.connect_ms.or_else(|| {
            perf_data.get("connect").and_then(|v| v.as_f64()).filter(|&x| x > 0.0)
        });
        let nav_ttfb = perf_data.get("ttfb").and_then(|v| v.as_f64()).filter(|&x| x > 0.0);
        let nav_lcp = perf_data.get("lcp").and_then(|v| v.as_f64()).filter(|&x| x > 0.0);
        let nav_fp = perf_data.get("fp").and_then(|v| v.as_f64()).filter(|&x| x > 0.0);
        let nav_fcp = perf_data.get("fcp").and_then(|v| v.as_f64()).filter(|&x| x > 0.0);
        let nav_dcl = perf_data.get("dcl").and_then(|v| v.as_f64()).filter(|&x| x > 0.0);

        BrowserResult {
            fp_ms: nav_fp.or(pg.first_paint),
            fcp_ms: nav_fcp.or(pg.first_contentful_paint),
            dom_content_loaded_ms: nav_dcl.or(pg.dom_content_loaded_ms),
            load_event_ms: pg.load_event_ms,
            page_open_time_ms: Some(nav_elapsed),
            first_paint_ms: nav_fp,
            resource_count: Some(net_metrics.request_count),
            resource_total_size: Some(net_metrics.total_transfer_size as i32),
            final_url, page_title: title, screenshot,
            error: None,
            html_size: Some(net_metrics.html_size as i32),
            css_size: Some(net_metrics.css_size as i32),
            js_size: Some(net_metrics.js_size as i32),
            image_size: Some(net_metrics.image_size as i32),
            font_size: Some(net_metrics.font_size as i32),
            total_requests: Some(net_metrics.request_count),
            failed_requests: Some(net_metrics.failed_count),
            lcp_ms: nav_lcp,
            cls: None,
            tti_ms: None,
            nav_dns_ms: nav_dns, nav_connect_ms: nav_connect, nav_ttfb_ms: nav_ttfb,
            site_size_kb: Some(net_metrics.site_size_kb),
            avg_speed_kbps: Some(net_metrics.avg_speed_kbps),
            total_speed_kbps: Some(net_metrics.total_speed_kbps),
            first_screen_ratio: Some(net_metrics.first_screen_ratio),
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
        nav_dns_ms: None, nav_connect_ms: None, nav_ttfb_ms: None,
        site_size_kb: None, avg_speed_kbps: None, total_speed_kbps: None, first_screen_ratio: None,
    }
}
