pub mod generic;
pub mod bilibili;
pub mod youtube;
pub mod registry;

use chromiumoxide::page::Page;

#[derive(Debug, Clone)]
pub struct PlayerInfo {
    pub platform: String,
    pub player_type: String,
    pub video_selector: Option<String>,
    pub play_trigger_js: Option<String>,
}

/// 播放器适配器 trait — 新增平台只需实现此接口
#[async_trait::async_trait]
pub trait PlayerAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    async fn detect(&self, page: &Page, url: &str) -> bool;
    fn video_selectors(&self) -> Vec<String>;
    fn play_trigger_js(&self) -> Option<String>;
}
