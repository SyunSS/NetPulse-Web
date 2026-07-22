use anyhow::Context;
use chromiumoxide::{Browser, BrowserConfig as ChromeBrowserConfig};
use chromiumoxide::page::Page;
use futures::StreamExt;
use tracing::info;

use crate::config::VideoBrowserConfig;

pub struct ChromiumoxideBrowser {
    browser: Browser,
}

impl ChromiumoxideBrowser {
    pub async fn launch(config: &VideoBrowserConfig) -> anyhow::Result<Self> {
        info!("Chromiumoxide 启动: path={}, headless={}", config.path, config.headless);

        let mut builder = ChromeBrowserConfig::builder()
            .no_sandbox()
            .window_size(1920, 1080)
            .arg("--autoplay-policy=no-user-gesture-required")
            .arg("--mute-audio")
            .arg("--disable-features=PreloadMediaEngagementData,MediaEngagementBypassAutoplayPolicies")
            .arg("--disable-gpu")
            .arg("--log-level=0")
            .chrome_executable(&config.path);

        if !config.headless {
            builder = builder.with_head();
        }

        let launch_config = builder
            .build()
            .map_err(|e| anyhow::anyhow!("构建 BrowserConfig 失败: {}", e))?;

        let (browser, mut handler) = Browser::launch(launch_config).await
            .context("Chromiumoxide 启动失败")?;

        // spawn handler in background
        tokio::spawn(async move {
            loop {
                let _ = handler.next().await;
            }
        });

        info!("Chromiumoxide 启动成功");
        Ok(Self { browser })
    }

    pub async fn new_page(&self) -> anyhow::Result<Page> {
        self.browser.new_page("about:blank").await
            .context("创建页面失败")
    }
}
