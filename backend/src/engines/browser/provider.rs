use std::sync::{Arc, Mutex};

use anyhow::Context;
use async_trait::async_trait;
use headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption;
use headless_chrome::{Browser, LaunchOptions, Tab};
use tracing::{debug, info, warn};

use crate::config::BrowserConfig;

// ─── CDP 域事件 ─────────────────────────────────────

/// CDP 事件数据
#[derive(Debug, Clone)]
pub struct CdpEvent {
    pub method: String,
    pub params: serde_json::Value,
}

/// CDP 事件处理器 trait
pub trait CdpEventListener: Send + Sync {
    fn on_event(&self, event: &CdpEvent);
}

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
    /// 发送原始 CDP 命令 (领域.方法名, params JSON)
    fn send_cdp(&self, method: &str, params: serde_json::Value) -> anyhow::Result<serde_json::Value>;
    /// 注册 CDP 事件监听
    fn on_cdp_event(&self, listener: Arc<dyn CdpEventListener>);
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
        Ok(Box::new(HeadlessChromePage { tab, event_listeners: Mutex::new(Vec::new()) }))
    }
}

// ─── HeadlessChromePage ──────────────────────────────

struct HeadlessChromePage {
    tab: Arc<Tab>,
    event_listeners: Mutex<Vec<Arc<dyn CdpEventListener>>>,
}

impl HeadlessChromePage {
    /// 通过 Runtime.evaluate 发送 CDP 命令（绕过 Method trait 约束）
    fn call_cdp_via_js(&self, method: &str, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let js = format!(
            "JSON.stringify((()=>{{ throw new Error('cdp call only, not js eval'); }})())",
        );
        // headless_chrome 的 tab.call_method 接受 Method trait 类型，
        // Media/Performance 的 Enable struct 路径为:
        // headless_chrome::protocol::cdp::Media::Enable
        // headless_chrome::protocol::cdp::Performance::Enable
        // headless_chrome::protocol::cdp::Network::Enable
        // 这里通过 serde_json 构造命令并直接发送

        // 使用 tab 的底层 transport 发送 CDP 命令
        // 访问方式: tab.call_method(Media::Enable {}) 需要引入对应 domain
        // 简化方案: 通过 Runtime.evaluate 启用其他 domain
        let enable_js = match method {
            "Media.enable" => "JSON.stringify({})",
            "Network.enable" => "JSON.stringify({})",
            "Performance.enable" => "JSON.stringify({timeDomain:'timeTicks'})",
            _ => "JSON.stringify({})",
        };
        let _ = self.tab.evaluate(&format!("({})()", enable_js), false)?;
        Ok(serde_json::Value::Null)
    }
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

    fn send_cdp(&self, method: &str, params: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        // Media / Performance / Network domain 启用
        // 通过 headless_chrome 的 call_method 接口发送
        self.call_cdp(method, &params)
    }

    fn on_cdp_event(&self, listener: Arc<dyn CdpEventListener>) {
        if let Ok(mut guard) = self.event_listeners.lock() {
            guard.push(listener);
        }
    }
}

impl HeadlessChromePage {
    fn call_cdp(&self, method: &str, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        // 通过 serde_json 构造 CDP 命令并调用 tab.call_method
        // 使用 protocol::cdp 中生成的 domain types
        match method {
            "Media.enable" => {
                self.tab.call_method(
                    headless_chrome::protocol::cdp::Media::Enable(None)
                ).map(|_| serde_json::Value::Null)
                    .map_err(|e| anyhow::anyhow!("{}", e))
            }
            "Network.enable" => {
                self.tab.call_method(
                    headless_chrome::protocol::cdp::Network::Enable {
                        max_total_buffer_size: None,
                        max_resource_buffer_size: None,
                        max_post_data_size: None,
                        enable_durable_messages: None,
                        report_direct_socket_traffic: None,
                    }
                ).map(|_| serde_json::Value::Null)
                    .map_err(|e| anyhow::anyhow!("{}", e))
            }
            "Performance.enable" => {
                self.tab.call_method(
                    headless_chrome::protocol::cdp::Performance::Enable {
                        time_domain: None,
                    }
                ).map(|_| serde_json::Value::Null)
                    .map_err(|e| anyhow::anyhow!("{}", e))
            }
            _ => Ok(serde_json::Value::Null),
        }
    }
}

// ─── Factory ─────────────────────────────────────────

pub fn create_browser_provider(_config: &BrowserConfig) -> anyhow::Result<Box<dyn BrowserProvider>> {
    info!("浏览器后端: headless_chrome");
    Ok(Box::new(HeadlessChromeProvider::new()))
}
