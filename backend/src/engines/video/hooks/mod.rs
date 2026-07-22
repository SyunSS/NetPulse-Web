pub mod media_element;
pub mod network_api;
pub mod media_source;
pub mod mutation;

use chromiumoxide::page::{Page, ScreenshotParams};
use tracing::info;

pub struct JSHookManager {
    page: Page,
}

impl JSHookManager {
    pub fn new(page: Page) -> Self {
        Self { page }
    }

    pub async fn inject_all(&self) -> anyhow::Result<()> {
        info!("开始注入 JS Hooks...");

        let hooks_js = vec![
            media_element::hook_script(),
            network_api::hook_script(),
            media_source::hook_script(),
            mutation::hook_script(),
        ];

        for (i, script) in hooks_js.iter().enumerate() {
            let result = self.page.evaluate(*script).await;
            match result {
                Ok(_) => info!("Hook #{}/{} 注入成功", i + 1, hooks_js.len()),
                Err(e) => info!("Hook #{}/{} 注入失败: {}", i + 1, hooks_js.len(), e),
            }
        }

        info!("{} 个 JS Hook 注入完成", hooks_js.len());
        Ok(())
    }

    pub async fn trigger_play(&self, extra_js: Option<&str>) -> anyhow::Result<()> {
        let js = r#"
        (function(){
            var videos = document.querySelectorAll('video');
            for (var i=0; i<videos.length; i++) {
                try { videos[i].play().catch(function(){}); } catch(e){}
            }
        })()
        "#;
        let _ = self.page.evaluate(js).await;

        if let Some(extra) = extra_js {
            let _ = self.page.evaluate(extra).await;
        }
        Ok(())
    }

    pub async fn click_center(&self) -> anyhow::Result<()> {
        let js = "document.elementFromPoint(window.innerWidth/2,window.innerHeight/2)?.click()";
        let _ = self.page.evaluate(js).await;
        Ok(())
    }

    pub async fn poll_video_state(&self) -> anyhow::Result<serde_json::Value> {
        let js = r#"
        (function(){
            var videos = document.querySelectorAll('video');
            if (videos.length === 0) return JSON.stringify({alive:false,count:0});
            var v = videos[0];
            return JSON.stringify({
                alive: true, count: videos.length,
                ct: v.currentTime, paused: v.paused, ended: v.ended,
                readyState: v.readyState, networkState: v.networkState,
                vw: v.videoWidth, vh: v.videoHeight, vdur: v.duration,
                webkitDecoded: v.webkitDecodedFrameCount || 0,
                webkitDropped: v.webkitDroppedFrameCount || 0
            });
        })()
        "#;
        let result = self.page.evaluate(js).await?;
        let text: String = result.into_value().unwrap_or_default();
        let parsed = serde_json::from_str(&text).unwrap_or(serde_json::json!({}));
        Ok(parsed)
    }

    pub async fn detect_video_elements(&self) -> anyhow::Result<u32> {
        let js = "document.querySelectorAll('video').length";
        let result = self.page.evaluate(js).await?;
        let count: u64 = result.into_value().unwrap_or(0);
        Ok(count as u32)
    }

    pub async fn page_title(&self) -> anyhow::Result<String> {
        let result = self.page.evaluate("document.title").await?;
        let title: String = result.into_value().unwrap_or_default();
        Ok(title)
    }

    pub async fn screenshot(&self) -> anyhow::Result<Vec<u8>> {
        let params = ScreenshotParams::builder()
            .full_page(true)
            .build();
        let screenshot = self.page.screenshot(params).await
            .map_err(|e| anyhow::anyhow!("截图失败: {}", e))?;
        Ok(screenshot)
    }

    pub async fn dismiss_popups(&self) -> anyhow::Result<()> {
        let js = r#"
        (function(){
            var btns = document.querySelectorAll('button,a,[role=button]');
            var keywords = ['accept','agree','allow','ok','yes','同意','接受','允许','确定','继续','关闭','close','dismiss','got it','skip','later','稍后'];
            for (var i=0; i<btns.length; i++) {
                var t = (btns[i].textContent||'').toLowerCase();
                for (var j=0; j<keywords.length; j++) {
                    if (t.includes(keywords[j])) { try { btns[i].click(); } catch(e){} break; }
                }
            }
        })()
        "#;
        let _ = self.page.evaluate(js).await;
        Ok(())
    }
}
