pub mod media;
pub mod network;
pub mod performance;
pub mod runtime;
pub mod page;

use std::sync::Arc;

use chromiumoxide::page::Page;
use tracing::info;

use super::events::VideoEvent;

pub struct CdpManager {
    page: Arc<Page>,
}

impl CdpManager {
    pub fn new(page: Arc<Page>) -> Self {
        Self { page }
    }

    pub async fn enable_all(&self) -> anyhow::Result<()> {
        info!("启用 CDP 域: Media, Network, Performance, Runtime, Page");

        let cdp_cmds = vec![
            "Media.enable",
            "Network.enable",
            "Performance.enable",
            "Runtime.enable",
            "Page.enable",
        ];

        for cmd in &cdp_cmds {
            match self.page.execute(cmd).await {
                Ok(_) => info!("CDP {} 启用成功", cmd),
                Err(e) => info!("CDP {} 启用失败: {}", cmd, e),
            }
        }
        Ok(())
    }

    pub fn page(&self) -> &Page {
        &self.page
    }
}
