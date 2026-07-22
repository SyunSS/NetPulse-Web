use chromiumoxide::page::Page;

use super::PlayerAdapter;

pub struct BilibiliAdapter;

impl BilibiliAdapter {
    pub fn new() -> Self { Self }
}

#[async_trait::async_trait]
impl PlayerAdapter for BilibiliAdapter {
    fn name(&self) -> &'static str { "bilibili" }

    async fn detect(&self, _page: &Page, url: &str) -> bool {
        let lower = url.to_lowercase();
        lower.contains("bilibili.com") || lower.contains("b23.tv")
    }

    fn video_selectors(&self) -> Vec<String> {
        vec![
            "video.bpx-player-video".to_string(),
            "video".to_string(),
        ]
    }

    fn play_trigger_js(&self) -> Option<String> {
        Some(r#"
        (function(){
            var v = document.querySelector('video');
            if (v) { try { v.play().catch(function(){}); } catch(e){} }
            var btn = document.querySelector('.bpx-player-ctrl-play');
            if (btn) { try { btn.click(); } catch(e){} }
        })()
        "#.to_string())
    }
}
