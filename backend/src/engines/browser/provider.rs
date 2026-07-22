use std::sync::Arc;

use anyhow::Context;
use headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption;
use headless_chrome::{Browser, LaunchOptions, Tab};
use tracing::debug;

use crate::config::BrowserConfig;

pub struct ChromePage {
    tab: Arc<Tab>,
}

impl ChromePage {
    pub fn navigate_to(&self, url: &str) -> anyhow::Result<()> {
        self.tab.navigate_to(url).context("导航失败")?;
        debug!("导航完成");
        Ok(())
    }

    pub fn wait_for_load(&self) -> anyhow::Result<()> {
        self.tab.wait_until_navigated().context("等待页面加载失败")?;
        Ok(())
    }

    pub fn evaluate_sync(&self, js: &str) -> anyhow::Result<serde_json::Value> {
        let result = self.tab.evaluate(js, false).context("JS 执行失败")?;
        Ok(result.value.clone().unwrap_or(serde_json::Value::Null))
    }

    pub fn screenshot(&self) -> anyhow::Result<Vec<u8>> {
        self.tab
            .capture_screenshot(CaptureScreenshotFormatOption::Png, None, None, true)
            .context("截图失败")
    }

    pub fn send_cdp(&self, method: &str, _params: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        match method {
            "Network.enable" => {
                self.tab.call_method(
                    headless_chrome::protocol::cdp::Network::Enable {
                        max_total_buffer_size: None, max_resource_buffer_size: None,
                        max_post_data_size: None, enable_durable_messages: None,
                        report_direct_socket_traffic: None,
                    }
                ).map(|_| serde_json::Value::Null).map_err(|e| anyhow::anyhow!("{}", e))
            }
            "Media.enable" => {
                self.tab.call_method(
                    headless_chrome::protocol::cdp::Media::Enable(None)
                ).map(|_| serde_json::Value::Null).map_err(|e| anyhow::anyhow!("{}", e))
            }
            _ => Ok(serde_json::Value::Null),
        }
    }
}

pub fn launch_browser(config: &BrowserConfig) -> anyhow::Result<Browser> {
    let path_buf = std::path::PathBuf::from(&config.path);
    let options = LaunchOptions::default_builder()
        .headless(config.headless)
        .sandbox(false)
        .enable_gpu(false)
        .enable_logging(false)
        .window_size(Some((1920, 1080)))
        .path(Some(path_buf))
        .build()
        .context("构建 LaunchOptions 失败")?;

    let browser = Browser::new(options).context("浏览器启动失败")?;
    debug!("HeadlessChrome 已启动");
    Ok(browser)
}

pub fn new_page(browser: &Browser) -> anyhow::Result<ChromePage> {
    let tab = browser.new_tab().context("创建标签页失败")?;
    debug!("新标签页已创建");
    Ok(ChromePage { tab })
}
