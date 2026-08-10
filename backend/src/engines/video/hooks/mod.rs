pub mod media_element;
pub mod media_source;
pub mod mutation;
pub mod network_api;

use chromiumoxide::layout::Point;
use chromiumoxide::page::{Page, ScreenshotParams};
use chromiumoxide::cdp::browser_protocol::network::{CookieParam, CookieSameSite, SetCookiesParams};
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

    pub async fn inject_on_new_document(&self) -> anyhow::Result<()> {
        let hooks_js = vec![
            media_element::hook_script(),
            network_api::hook_script(),
            media_source::hook_script(),
            mutation::hook_script(),
        ];
        for script in hooks_js {
            self.page
                .evaluate_on_new_document(script)
                .await
                .map_err(|e| anyhow::anyhow!("导航前 Hook 注入失败: {}", e))?;
        }
        Ok(())
    }

    pub async fn trigger_play(&self, extra_js: Option<&str>) -> anyhow::Result<()> {
        let js = r#"
        (function(){
            var videos = document.querySelectorAll('video');
            for (var i=0; i<videos.length; i++) {
                try { videos[i].muted = true; videos[i].play().catch(function(){}); } catch(e){}
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
        let point = Point {
            x: 960.0,
            y: 540.0,
        };
        let _ = self.page.click(point).await;
        Ok(())
    }

    pub async fn click_play_candidate(&self, selectors: &[String]) -> anyhow::Result<bool> {
        let selectors_json = serde_json::to_string(selectors)?;
        let js = format!(
            r#"
        (function(){{
            var selectors = {selectors_json};
            function visible(el) {{
                if (!el) return false;
                var style = window.getComputedStyle(el);
                if (style.visibility === 'hidden' || style.display === 'none' || style.pointerEvents === 'none') return false;
                var r = el.getBoundingClientRect();
                return r.width >= 8 && r.height >= 8 && r.bottom >= 0 && r.right >= 0 && r.left <= window.innerWidth && r.top <= window.innerHeight;
            }}
            function point(el) {{
                var r = el.getBoundingClientRect();
                return {{x: Math.max(1, Math.min(window.innerWidth - 1, r.left + r.width / 2)), y: Math.max(1, Math.min(window.innerHeight - 1, r.top + r.height / 2))}};
            }}
            for (var i = 0; i < selectors.length; i++) {{
                try {{
                    var el = document.querySelector(selectors[i]);
                    if (visible(el)) return JSON.stringify(point(el));
                }} catch(e) {{}}
            }}
            var keywords = ['播放','开始','play','start','继续','观看'];
            var candidates = document.querySelectorAll('button,a,[role=button],.btn,.play,.play-btn');
            for (var j = 0; j < candidates.length; j++) {{
                var text = ((candidates[j].textContent || '') + ' ' + (candidates[j].getAttribute('aria-label') || '') + ' ' + (candidates[j].getAttribute('title') || '')).toLowerCase();
                for (var k = 0; k < keywords.length; k++) {{
                    if (text.indexOf(keywords[k].toLowerCase()) >= 0 && visible(candidates[j])) return JSON.stringify(point(candidates[j]));
                }}
            }}
            var v = document.querySelector('video');
            if (visible(v)) return JSON.stringify(point(v));
            return JSON.stringify(null);
        }})()
        "#
        );
        let result = self.page.evaluate(js).await?;
        let text: String = result.into_value().unwrap_or_default();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
        let Some(x) = value.get("x").and_then(|v| v.as_f64()) else {
            return Ok(false);
        };
        let Some(y) = value.get("y").and_then(|v| v.as_f64()) else {
            return Ok(false);
        };
        let _ = self.page.click(Point { x, y }).await;
        Ok(true)
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

    pub async fn set_cookies_from_json(&self, cookies: &serde_json::Value) -> anyhow::Result<usize> {
        let Some(items) = cookies.as_array() else {
            return Ok(0);
        };
        let mut params = Vec::new();
        for item in items {
            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or_default();
            let value = item.get("value").and_then(|v| v.as_str()).unwrap_or_default();
            let domain = item.get("domain").and_then(|v| v.as_str()).unwrap_or_default();
            if name.is_empty() || domain.is_empty() {
                continue;
            }
            let mut builder = CookieParam::builder()
                .name(name.to_string())
                .value(value.to_string())
                .domain(domain.to_string())
                .path(
                    item.get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("/")
                        .to_string(),
                )
                .secure(item.get("secure").and_then(|v| v.as_bool()).unwrap_or(false))
                .http_only(item.get("httpOnly").and_then(|v| v.as_bool()).unwrap_or(false));
            if let Some(url) = item.get("url").and_then(|v| v.as_str()).filter(|v| !v.is_empty()) {
                builder = builder.url(url.to_string());
            }
            if let Some(same_site) = item.get("sameSite").and_then(|v| v.as_str()).and_then(parse_same_site) {
                builder = builder.same_site(same_site);
            }
            if let Ok(cookie) = builder.build() {
                params.push(cookie);
            }
        }
        let count = params.len();
        if count > 0 {
            self.page.execute(SetCookiesParams::new(params)).await?;
        }
        Ok(count)
    }

    pub async fn page_is_usable(&self) -> anyhow::Result<bool> {
        let js = r#"
        (function(){
            return !!(location.href !== 'about:blank' && document.body && (document.querySelector('video') || document.body.innerText.length > 100));
        })()
        "#;
        let result = self.page.evaluate(js).await?;
        Ok(result.into_value().unwrap_or(false))
    }

    pub async fn page_title(&self) -> anyhow::Result<String> {
        let result = self.page.evaluate("document.title").await?;
        let title: String = result.into_value().unwrap_or_default();
        Ok(title)
    }

    pub async fn page_text(&self) -> anyhow::Result<String> {
        let result = self
            .page
            .evaluate("document.body ? document.body.innerText.slice(0, 12000) : ''")
            .await?;
        let text: String = result.into_value().unwrap_or_default();
        Ok(text)
    }

    pub async fn playback_diagnostic_text(&self) -> anyhow::Result<String> {
        let js = r#"
        (function(){
            var parts = [];
            function add(el) {
                if (!el) return;
                var style = window.getComputedStyle(el);
                if (style.display === 'none' || style.visibility === 'hidden') return;
                var text = (el.innerText || el.textContent || '').trim();
                if (text) parts.push(text.slice(0, 2500));
            }
            document.querySelectorAll('[role=dialog],.modal,.popup,.dialog,.error,.login,.paywall,.vip,.captcha,.verify').forEach(add);
            var video = document.querySelector('video');
            var parent = video;
            for (var i = 0; i < 4 && parent; i++, parent = parent.parentElement) add(parent);
            return parts.join('\n').slice(0, 12000);
        })()
        "#;
        let result = self.page.evaluate(js).await?;
        Ok(result.into_value().unwrap_or_default())
    }

    pub async fn screenshot(&self) -> anyhow::Result<Vec<u8>> {
        let params = ScreenshotParams::builder().full_page(true).build();
        let screenshot = self
            .page
            .screenshot(params)
            .await
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

fn parse_same_site(value: &str) -> Option<CookieSameSite> {
    match value.trim().to_ascii_lowercase().as_str() {
        "strict" => Some(CookieSameSite::Strict),
        "lax" => Some(CookieSameSite::Lax),
        "none" | "no_restriction" => Some(CookieSameSite::None),
        _ => None,
    }
}
