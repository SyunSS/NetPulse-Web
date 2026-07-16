use std::ffi::OsStr;
use std::sync::Arc;

use anyhow::Context;
use async_trait::async_trait;
use headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption;
use headless_chrome::{Browser, LaunchOptions, Tab};
use tracing::{debug, info};

use crate::config::BrowserConfig;

// ─── Traits ──────────────────────────────────────────

/// 浏览器提供者 — 负责启动浏览器进程
#[async_trait]
pub trait BrowserProvider: Send + Sync {
    async fn launch(
        &self,
        path: String,
        headless: bool,
        extra_args: Vec<String>,
    ) -> anyhow::Result<Box<dyn BrowserHandle>>;
}

/// 浏览器句柄 — 管理标签页/页面
#[async_trait]
pub trait BrowserHandle: Send {
    async fn new_page(&self) -> anyhow::Result<Box<dyn BrowserPage>>;
}

/// 浏览器页面 — 操作单个标签页
#[async_trait]
pub trait BrowserPage: Send {
    async fn navigate_to(&self, url: &str) -> anyhow::Result<()>;
    async fn wait_for_load(&self) -> anyhow::Result<()>;
    fn evaluate_sync(&self, js: &str) -> anyhow::Result<serde_json::Value>;
    fn screenshot(&self) -> anyhow::Result<Vec<u8>>;
}

// ─── HeadlessChromeProvider ───────────────────────────

pub struct HeadlessChromeProvider;

impl HeadlessChromeProvider {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl BrowserProvider for HeadlessChromeProvider {
    async fn launch(
        &self,
        path: String,
        headless: bool,
        _extra_args: Vec<String>,
    ) -> anyhow::Result<Box<dyn BrowserHandle>> {
        let path_buf = std::path::PathBuf::from(&path);

        // headless Chrome 默认无磁盘缓存，加 --incognito 确保隔离
        let options = LaunchOptions::default_builder()
            .headless(headless)
            .sandbox(false)
            .enable_gpu(false)
            .enable_logging(false)
            .window_size(Some((1920, 1080)))
            .path(Some(path_buf))
            .build()
            .context("构建 LaunchOptions 失败")?;

        let browser = tokio::task::spawn_blocking(move || Browser::new(options))
            .await.context("spawn_blocking 失败")?
            .context("浏览器启动失败")?;

        debug!("HeadlessChrome 已启动 (无缓存模式)");
        Ok(Box::new(HeadlessChromeHandle { browser: Arc::new(browser) }))
    }
}

// ─── HeadlessChromeHandle ────────────────────────────

struct HeadlessChromeHandle {
    browser: Arc<Browser>,
}

#[async_trait]
impl BrowserHandle for HeadlessChromeHandle {
    async fn new_page(&self) -> anyhow::Result<Box<dyn BrowserPage>> {
        let browser = self.browser.clone();
        let tab = tokio::task::spawn_blocking(move || {
            browser.new_tab().context("创建标签页失败")
        })
        .await.context("spawn_blocking 失败")??;

        debug!("新标签页已创建");
        Ok(Box::new(HeadlessChromePage { tab }))
    }
}

// ─── HeadlessChromePage ──────────────────────────────

struct HeadlessChromePage {
    tab: Arc<Tab>,
}

#[async_trait]
impl BrowserPage for HeadlessChromePage {
    async fn navigate_to(&self, url: &str) -> anyhow::Result<()> {
        let u = url.to_string();
        let tab = self.tab.clone();
        tokio::task::spawn_blocking(move || {
            tab.navigate_to(&u).context("导航失败")?;
            Ok::<_, anyhow::Error>(())
        })
        .await.context("spawn_blocking 失败")??;
        debug!("导航完成");
        Ok(())
    }

    async fn wait_for_load(&self) -> anyhow::Result<()> {
        let tab = self.tab.clone();
        tokio::task::spawn_blocking(move || {
            tab.wait_until_navigated().context("等待页面加载失败")?;
            Ok::<_, anyhow::Error>(())
        })
        .await.context("spawn_blocking 失败")??;
        Ok(())
    }

    fn evaluate_sync(&self, js: &str) -> anyhow::Result<serde_json::Value> {
        let result = self.tab.evaluate(js, false).context("JS 执行失败")?;
        Ok(result.value.clone().unwrap_or(serde_json::Value::Null))
    }

    fn screenshot(&self) -> anyhow::Result<Vec<u8>> {
        self.tab
            .capture_screenshot(CaptureScreenshotFormatOption::Png, None, None, true)
            .context("截图失败")
    }
}

// ─── ChromiumoxideProvider (桩) ──────────────────────

pub struct ChromiumoxideProvider;

#[async_trait]
impl BrowserProvider for ChromiumoxideProvider {
    async fn launch(
        &self,
        _path: String,
        _headless: bool,
        extra_args: Vec<String>,
    ) -> anyhow::Result<Box<dyn BrowserHandle>> {
        anyhow::bail!("chromiumoxide 后端尚未实现，请使用 headless_chrome")
    }
}

// ─── Factory ─────────────────────────────────────────

pub fn create_browser_provider(config: &BrowserConfig) -> anyhow::Result<Box<dyn BrowserProvider>> {
    match config.provider.as_str() {
        "headless_chrome" => {
            info!("浏览器后端: headless_chrome");
            Ok(Box::new(HeadlessChromeProvider::new()))
        }
        "chromiumoxide" => {
            info!("浏览器后端: chromiumiumoxide (桩)");
            Ok(Box::new(ChromiumoxideProvider))
        }
        other => anyhow::bail!("不支持的浏览器后端: {}，可用: headless_chrome", other),
    }
}
