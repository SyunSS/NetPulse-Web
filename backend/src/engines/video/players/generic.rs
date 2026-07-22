use chromiumoxide::page::Page;

use super::PlayerAdapter;

pub struct GenericHtml5Adapter;

impl GenericHtml5Adapter {
    pub fn new() -> Self { Self }
}

#[async_trait::async_trait]
impl PlayerAdapter for GenericHtml5Adapter {
    fn name(&self) -> &'static str { "html5" }

    async fn detect(&self, page: &Page, _url: &str) -> bool {
        let result = page.evaluate("document.querySelectorAll('video').length > 0").await;
        match result {
            Ok(r) => r.into_value::<bool>().unwrap_or(false),
            Err(_) => false,
        }
    }

    fn video_selectors(&self) -> Vec<String> {
        vec!["video".to_string()]
    }

    fn play_trigger_js(&self) -> Option<String> {
        Some(r#"
        (function(){
            var videos = document.querySelectorAll('video');
            for (var i=0; i<videos.length; i++) {
                try { videos[i].play().catch(function(){}); } catch(e){}
            }
        })()
        "#.to_string())
    }
}
