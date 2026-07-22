use chromiumoxide::page::Page;

use super::PlayerAdapter;

pub struct YoutubeAdapter;

impl YoutubeAdapter {
    pub fn new() -> Self { Self }
}

#[async_trait::async_trait]
impl PlayerAdapter for YoutubeAdapter {
    fn name(&self) -> &'static str { "youtube" }

    async fn detect(&self, _page: &Page, url: &str) -> bool {
        let lower = url.to_lowercase();
        lower.contains("youtube.com") || lower.contains("youtu.be")
    }

    fn video_selectors(&self) -> Vec<String> {
        vec![
            "video.html5-main-video".to_string(),
            "video".to_string(),
        ]
    }

    fn play_trigger_js(&self) -> Option<String> {
        Some(r#"
        (function(){
            var v = document.querySelector('video.html5-main-video');
            if (v) {
                try { v.muted = true; v.play().catch(function(){}); } catch(e){}
            }
            var btn = document.querySelector('.ytp-play-button');
            if (btn) { try { btn.click(); } catch(e){} }
        })()
        "#.to_string())
    }
}
