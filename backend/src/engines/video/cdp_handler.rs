use std::sync::Arc;

use crate::engines::browser::provider::{CdpEvent, CdpEventListener};
use crate::engines::video::collector::{MediaCollector, NetworkCollector};

/// CDP 事件分发器 — 将 headless_chrome 事件路由到各个 Collector
pub struct VideoCdpHandler {
    pub media: Arc<MediaCollector>,
    pub network: Arc<NetworkCollector>,
}

impl VideoCdpHandler {
    pub fn new() -> Self {
        Self {
            media: Arc::new(MediaCollector::new()),
            network: Arc::new(NetworkCollector::new()),
        }
    }
}

impl CdpEventListener for VideoCdpHandler {
    fn on_event(&self, event: &CdpEvent) {
        let method = &event.method;
        if method.starts_with("Media.") {
            self.media.handle_cdp(event);
        }
        if method.starts_with("Network.") {
            self.network.handle_cdp(event);
        }
    }
}
