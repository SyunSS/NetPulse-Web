use std::sync::Mutex;

use serde::Deserialize;

/// CDP Page 域采集的数据
#[derive(Debug, Clone, Default)]
pub struct PageMetrics {
    pub dom_content_loaded_ms: Option<f64>,
    pub load_event_ms: Option<f64>,
    pub first_paint: Option<f64>,
    pub first_contentful_paint: Option<f64>,
    pub navigation_start: Option<f64>,
}

/// CDP Network 域采集的数据
#[derive(Debug, Clone, Default)]
pub struct NetworkMetrics {
    pub request_count: i32,
    pub failed_count: i32,
    pub total_transfer_size: u64,
    pub html_size: u64,
    pub css_size: u64,
    pub js_size: u64,
    pub image_size: u64,
    pub font_size: u64,
    pub media_size: u64,
}

/// PageCollector — 监听 Page 域事件
pub struct PageCollector {
    pub data: Mutex<PageMetrics>,
    start: std::time::Instant,
}

impl PageCollector {
    pub fn new() -> Self {
        Self { data: Mutex::new(PageMetrics::default()), start: std::time::Instant::now() }
    }

    pub fn record_navigation(&self) {
        if let Ok(mut m) = self.data.lock() {
            m.navigation_start = Some(self.start.elapsed().as_secs_f64() * 1000.0);
        }
    }

    pub fn record_load(&self) {
        if let Ok(mut m) = self.data.lock() {
            let elapsed = self.start.elapsed().as_secs_f64() * 1000.0;
            if let Some(nav_start) = m.navigation_start {
                m.load_event_ms = Some(elapsed - nav_start);
                m.dom_content_loaded_ms = Some((elapsed - nav_start) * 0.7); // 估算
            }
        }
    }

    pub fn snapshot(&self) -> PageMetrics {
        self.data.lock().map(|m| m.clone()).unwrap_or_default()
    }
}

/// NetworkCollector — 通过 JS evaluate 采集资源统计
pub struct NetworkCollector;

impl NetworkCollector {
    pub fn new() -> Self { Self }

    pub fn collect_js() -> &'static str {
        r#"JSON.stringify((function(){
            var res = performance.getEntriesByType('resource');
            var r = { count: res.length, total: 0, html: 0, css: 0, js: 0, img: 0, font: 0, media: 0, failed: 0 };
            for (var i=0; i<res.length; i++) {
                var e = res[i];
                var s = e.transferSize || e.encodedBodySize || 0;
                r.total += s;
                if (/\.html|\.htm|text\/html/.test(e.name)) r.html += s;
                else if (/\.css|text\/css/.test(e.name)) r.css += s;
                else if (/\.js|text\/javascript/.test(e.name)) r.js += s;
                else if (/\.png|\.jpg|\.gif|\.svg|\.webp|\.ico|image\//.test(e.name)) r.img += s;
                else if (/\.woff|\.ttf|\.otf|font\//.test(e.name)) r.font += s;
                else r.media += s;
            }
            return r;
        })())"#
    }

    pub fn parse(data: serde_json::Value) -> NetworkMetrics {
        serde_json::from_value::<RawNetwork>(data).map(|r| NetworkMetrics {
            request_count: r.count, failed_count: r.failed,
            total_transfer_size: r.total as u64,
            html_size: r.html as u64, css_size: r.css as u64,
            js_size: r.js as u64, image_size: r.img as u64,
            font_size: r.font as u64, media_size: r.media as u64,
        }).unwrap_or_default()
    }
}

#[derive(Debug, Deserialize)]
struct RawNetwork {
    count: i32, total: f64, html: f64, css: f64, js: f64, img: f64, font: f64, media: f64,
    #[serde(default)] failed: i32,
}
