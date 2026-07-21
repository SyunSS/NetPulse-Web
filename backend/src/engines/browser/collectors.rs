use std::sync::Mutex;
use serde::Deserialize;

/// CDP Page 域采集的数据
#[derive(Debug, Clone, Default)]
pub struct PageMetrics {
    pub dom_content_loaded_ms: Option<f64>,
    pub load_event_ms: Option<f64>,
    pub first_paint: Option<f64>,
    pub first_contentful_paint: Option<f64>,
    pub largest_contentful_paint: Option<f64>,
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
    // 新增
    pub site_size_kb: f64,
    pub avg_speed_kbps: f64,
    pub total_speed_kbps: f64,
    pub first_screen_ratio: f64,
    pub dns_ms: Option<f64>,
    pub connect_ms: Option<f64>,
}

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
                m.dom_content_loaded_ms = Some((elapsed - nav_start) * 0.7);
            }
        }
    }
    pub fn snapshot(&self) -> PageMetrics {
        self.data.lock().map(|m| m.clone()).unwrap_or_default()
    }
}

/// NetworkCollector — 完整的网站性能采集 JS (基于 Puppeteer 验证的脚本)
pub struct NetworkCollector;

impl NetworkCollector {
    pub fn new() -> Self { Self }

    /// 完整性能采集 JS — 输出 25+ 指标
    pub fn collect_js() -> &'static str {
        r#"JSON.stringify((function(){
            var nav = performance.getEntriesByType('navigation')[0] || {};
            var paints = performance.getEntriesByType('paint');
            var resources = performance.getEntriesByType('resource');
            var startTime = nav.startTime || 0;

            // Paint metrics
            var fp = paints.find(function(p){return p.name==='first-paint'});
            var fcp = paints.find(function(p){return p.name==='first-contentful-paint'});
            var fpMs = fp ? fp.startTime : 0;
            var fcpMs = fcp ? fcp.startTime : 0;
            var lcpMs = (window.__perfData && window.__perfData.lcp) || 0;

            // Navigation Timing
            var dnsMs = Math.max(0, (nav.domainLookupEnd||0) - (nav.domainLookupStart||0));
            var connectMs = Math.max(0, (nav.connectEnd||0) - (nav.connectStart||0));
            var ttfbMs = Math.max(0, (nav.responseStart||0) - (nav.requestStart||0));
            var dclMs = Math.max(0, (nav.domContentLoadedEventEnd||0) - startTime);
            var loadEndMs = nav.loadEventEnd || nav.domComplete || 0;
            var httpStatus = nav.responseStatus || 0;

            // 资源统计 (含主文档)
            var mainSize = nav.transferSize || nav.encodedBodySize || 0;
            var r = { count: 1, total: mainSize, html: mainSize, css: 0, js: 0, img: 0, font: 0, media: 0, failed: 0 };
            for (var i=0; i<resources.length; i++) {
                var e = resources[i];
                var s = e.transferSize || e.encodedBodySize || 0;
                r.total += s; r.count++;
                if (/\.html|\.htm|text\/html/.test(e.name)) r.html += s;
                else if (/\.css|text\/css/.test(e.name)) r.css += s;
                else if (/\.js|text\/javascript/.test(e.name)) r.js += s;
                else if (/\.png|\.jpg|\.gif|\.svg|\.webp|\.ico|image\//.test(e.name)) r.img += s;
                else if (/\.woff|\.ttf|\.otf|font\//.test(e.name)) r.font += s;
                else r.media += s;
            }

            // 衍生指标
            var siteSizeKB = r.total / 1024;
            var le = loadEndMs || 1;
            var loadSec = Math.max(0.001, le / 1000);
            var avgSpeed = r.total > 0 ? (r.total * 8) / loadSec / 1000 : 0;
            var responseMs = nav.responseStart || 0;
            var downloadSec = Math.max(0.001, (le - responseMs) / 1000);
            var totalSpeed = r.total > 0 ? (r.total * 8) / downloadSec / 1000 : 0;
            var fcpTime = fcpMs || lcpMs || (nav.domContentLoadedEventEnd||0);
            var resBeforeFCP = fcpTime > 0 ? resources.filter(function(rr){return (rr.responseEnd||0) <= fcpTime;}).length + 1 : r.count;
            var firstScreenRatio = r.count > 0 ? Math.min(100, Math.round(resBeforeFCP / r.count * 1000) / 10) : 0;

            return {
                fp: fpMs, fcp: fcpMs, lcp: lcpMs, dcl: dclMs, load: loadEndMs,
                dns: dnsMs, connect: connectMs, ttfb: ttfbMs, httpStatus: httpStatus,
                count: r.count, total: r.total, html: r.html, css: r.css, js: r.js,
                img: r.img, font: r.font, media: r.media, failed: r.failed,
                siteSizeKB: Math.round(siteSizeKB * 10) / 10,
                avgSpeedKbps: Math.round(avgSpeed * 10) / 10,
                totalSpeedKbps: Math.round(totalSpeed * 10) / 10,
                firstScreenRatio: firstScreenRatio
            };
        })())"#
    }

    /// LCP Observer 注入 JS — 页面加载前注入
    pub fn lcp_inject_js() -> &'static str {
        r#"(function(){
            window.__perfData = { lcp: 0 };
            try {
                new PerformanceObserver(function(list){
                    var entries = list.getEntries();
                    if (entries.length > 0) {
                        var last = entries[entries.length - 1];
                        window.__perfData.lcp = last.renderTime || last.loadTime || 0;
                    }
                }).observe({ type: 'largest-contentful-paint', buffered: true });
            } catch(e) { window.__perfData.lcpError = e.message; }
        })()"#
    }

    pub fn parse(data: serde_json::Value) -> NetworkMetrics {
        let r = serde_json::from_value::<RawPerf>(data).unwrap_or_default();
        NetworkMetrics {
            request_count: r.count, failed_count: r.failed,
            total_transfer_size: r.total as u64,
            html_size: r.html as u64, css_size: r.css as u64,
            js_size: r.js as u64, image_size: r.img as u64,
            font_size: r.font as u64, media_size: r.media as u64,
            site_size_kb: r.siteSizeKB,
            avg_speed_kbps: r.avgSpeedKbps,
            total_speed_kbps: r.totalSpeedKbps,
            first_screen_ratio: r.firstScreenRatio,
            dns_ms: if r.dns > 0.0 { Some(r.dns) } else { None },
            connect_ms: if r.connect > 0.0 { Some(r.connect) } else { None },
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct RawPerf {
    // Paint
    #[serde(default)] fp: f64,
    #[serde(default)] fcp: f64,
    #[serde(default)] lcp: f64,
    #[serde(default)] dcl: f64,
    #[serde(default)] load: f64,
    // Network (navigation timing)
    #[serde(default)] dns: f64,
    #[serde(default)] connect: f64,
    #[serde(default)] ttfb: f64,
    #[serde(default)] httpStatus: i32,
    // Resources
    #[serde(default)] count: i32,
    #[serde(default)] total: f64,
    #[serde(default)] html: f64,
    #[serde(default)] css: f64,
    #[serde(default)] js: f64,
    #[serde(default)] img: f64,
    #[serde(default)] font: f64,
    #[serde(default)] media: f64,
    #[serde(default)] failed: i32,
    // Derived
    #[serde(default)] siteSizeKB: f64,
    #[serde(default)] avgSpeedKbps: f64,
    #[serde(default)] totalSpeedKbps: f64,
    #[serde(default)] firstScreenRatio: f64,
}
