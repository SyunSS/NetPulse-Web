use std::path::PathBuf;

use chromiumoxide::page::Page;
use tracing::info;

use crate::config::VideoBrowserConfig;
use crate::engines::chromium::{ChromiumArg, ChromiumSession};

pub struct ChromiumoxideBrowser {
    session: ChromiumSession,
}

impl ChromiumoxideBrowser {
    pub async fn launch(config: &VideoBrowserConfig) -> anyhow::Result<Self> {
        info!(
            "Chromiumoxide 启动: path={}, headless={}",
            config.path, config.headless
        );

        let user_data_dir = config.user_data_dir.as_deref().map(PathBuf::from);
        let session = ChromiumSession::launch(
            "video",
            &config.path,
            config.headless,
            user_data_dir,
            &[
                ChromiumArg::Value("autoplay-policy", "no-user-gesture-required"),
                ChromiumArg::Key("mute-audio"),
                ChromiumArg::Value(
                    "disable-features",
                    "PreloadMediaEngagementData,MediaEngagementBypassAutoplayPolicies",
                ),
                ChromiumArg::Key("disable-gpu"),
            ],
        )
        .await?;

        Ok(Self { session })
    }

    pub async fn new_page(&self) -> anyhow::Result<Page> {
        self.session
            .browser()
            .new_page("about:blank")
            .await
            .map_err(|e| anyhow::anyhow!("创建页面失败: {:#}", e))
    }

    pub async fn shutdown(self) {
        self.session.shutdown().await;
    }
}
