use chromiumoxide::cdp::browser_protocol::performance::{
    EventMetrics,
};

pub struct PerformanceCollector;

impl PerformanceCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn handle_metrics(&self, _event: EventMetrics) {
        // 性能指标采集（预留扩展）
    }
}
