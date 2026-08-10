use anyhow::Context;
use chromiumoxide::page::{Page, ScreenshotParams};
use tracing::debug;

use crate::config::BrowserConfig;
use crate::engines::chromium::ChromiumSession;

pub struct ChromiumPage {
    page: Page,
}

impl ChromiumPage {
    pub async fn navigate_to(&self, url: &str) -> anyhow::Result<()> {
        self.page.goto(url).await.context("导航失败")?;
        debug!("导航完成");
        Ok(())
    }

    pub async fn evaluate(&self, js: &str) -> anyhow::Result<serde_json::Value> {
        let result = self.page.evaluate(js).await.context("JS 执行失败")?;
        let text: String = result.into_value().unwrap_or_default();
        // 尝试把 JS 返回的 JSON 字符串解析为 serde_json::Value
        if text.is_empty() {
            Ok(serde_json::Value::Null)
        } else {
            Ok(serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text)))
        }
    }

    pub async fn screenshot(&self) -> anyhow::Result<Vec<u8>> {
        let params = ScreenshotParams::builder().build();
        let screenshot = self.page.screenshot(params).await.context("截图失败")?;
        Ok(screenshot)
    }

    pub async fn wait_for_load(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

pub async fn launch_browser(config: &BrowserConfig) -> anyhow::Result<ChromiumSession> {
    ChromiumSession::launch("website", &config.path, config.headless, None, &[]).await
}

pub async fn new_page(session: &ChromiumSession) -> anyhow::Result<ChromiumPage> {
    let page = session
        .browser()
        .new_page("about:blank")
        .await
        .map_err(|e| anyhow::anyhow!("创建页面失败: {:#}", e))?;
    debug!("新页面已创建");
    Ok(ChromiumPage { page })
}
