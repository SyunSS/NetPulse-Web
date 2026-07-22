use std::sync::Arc;

use chromiumoxide::page::Page;
use tracing::info;

use super::{bilibili::BilibiliAdapter, generic::GenericHtml5Adapter, youtube::YoutubeAdapter, PlayerAdapter};

pub struct PlayerRegistry {
    adapters: Vec<Box<dyn PlayerAdapter>>,
}

impl PlayerRegistry {
    pub fn new() -> Self {
        let adapters: Vec<Box<dyn PlayerAdapter>> = vec![
            Box::new(BilibiliAdapter::new()),
            Box::new(YoutubeAdapter::new()),
            Box::new(GenericHtml5Adapter::new()),
        ];
        Self { adapters }
    }

    /// 检测并返回匹配的播放器适配器
    pub async fn detect(&self, page: &Page, url: &str) -> Option<&dyn PlayerAdapter> {
        for adapter in &self.adapters {
            if adapter.detect(page, url).await {
                info!("播放器识别: {}", adapter.name());
                return Some(adapter.as_ref());
            }
        }
        None
    }
}
