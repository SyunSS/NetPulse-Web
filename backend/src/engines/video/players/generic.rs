use chromiumoxide::page::Page;

use super::PlayerAdapter;

pub struct GenericHtml5Adapter;

impl GenericHtml5Adapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl PlayerAdapter for GenericHtml5Adapter {
    fn name(&self) -> &'static str {
        "html5"
    }

    async fn detect(&self, page: &Page, _url: &str) -> bool {
        let result = page
            .evaluate("document.querySelectorAll('video').length > 0")
            .await;
        match result {
            Ok(r) => r.into_value::<bool>().unwrap_or(false),
            Err(_) => false,
        }
    }

    fn video_selectors(&self) -> Vec<String> {
        vec!["video".to_string()]
    }

    fn play_button_selectors(&self) -> Vec<String> {
        vec![
            "video".to_string(),
            ".xgplayer-start".to_string(),
            ".xgplayer-play".to_string(),
            ".txp_btn_play".to_string(),
            ".txp_overlay_play".to_string(),
            ".mango-player .btn-play".to_string(),
            ".mgtv-player .btn-play".to_string(),
            ".ykplayer .x-play-btn".to_string(),
            ".youku-layer-logo".to_string(),
            ".play-btn".to_string(),
            ".btn-play".to_string(),
            "[aria-label*=\"play\" i]".to_string(),
            "[title*=\"play\" i]".to_string(),
            "[aria-label*=\"播放\"]".to_string(),
            "[title*=\"播放\"]".to_string(),
        ]
    }

    fn play_trigger_js(&self) -> Option<String> {
        Some(
            r#"
        (function(){
            var videos = document.querySelectorAll('video');
            for (var i=0; i<videos.length; i++) {
                try { videos[i].muted = true; videos[i].play().catch(function(){}); } catch(e){}
            }
        })()
        "#
            .to_string(),
        )
    }
}
