use std::time::{Duration, Instant};

use tracing::{debug, info};

/// 播放触发方式
#[derive(Debug, Clone)]
pub enum TriggerMethod {
    Auto,       // 页面自动播放
    Click,      // 模拟点击触发
    Keyboard,   // 键盘事件触发
    None,       // 未能触发
}

/// 播放控制器 — 多级播放触发 + 等待策略
pub struct PlaybackController {
    max_wait: Duration,
    check_interval: Duration,
    start: Instant,
    trigger_method: TriggerMethod,
    video_element_found: bool,
    media_event_count: u32,
    network_media_request_count: u32,
    last_error: Option<String>,
    player_created: bool,
}

impl PlaybackController {
    pub fn new(max_wait_secs: u64) -> Self {
        Self {
            max_wait: Duration::from_secs(max_wait_secs),
            check_interval: Duration::from_secs(2),
            start: Instant::now(),
            trigger_method: TriggerMethod::None,
            video_element_found: false,
            media_event_count: 0,
            network_media_request_count: 0,
            last_error: None,
            player_created: false,
        }
    }

    /// 运行完整的播放触发流程
    pub async fn run<F>(&mut self, do_click: F) -> TriggerMethod
    where
        F: Fn() + Send + 'static,
    {
        // Level 1: 等待自动播放 (0-12s)
        debug!("[PlaybackCtrl] Level 1: 等待自动播放");
        for _ in 0..6 {
            tokio::time::sleep(self.check_interval).await;
            if self.player_created || self.media_event_count > 2 {
                self.trigger_method = TriggerMethod::Auto;
                info!("[PlaybackCtrl] 自动播放触发成功 ({} 事件)", self.media_event_count);
                return self.trigger_method.clone();
            }
            if self.elapsed() > Duration::from_secs(12) { break; }
        }

        // Level 2: 模拟点击 (12-40s)
        debug!("[PlaybackCtrl] Level 2: 模拟点击触发");
        // 点击页面中心
        do_click();
        for _ in 0..6 {
            tokio::time::sleep(self.check_interval).await;
            if self.player_created || self.media_event_count > 2 {
                self.trigger_method = TriggerMethod::Click;
                info!("[PlaybackCtrl] 点击触发成功 ({} 事件)", self.media_event_count);
                return self.trigger_method.clone();
            }
            if self.elapsed() > Duration::from_secs(40) { break; }
        }

        // Level 3: 键盘事件 (40-60s)
        debug!("[PlaybackCtrl] Level 3: 键盘事件");
        do_click(); // re-attempt
        for _ in 0..10 {
            tokio::time::sleep(self.check_interval).await;
            if self.player_created || self.media_event_count > 2 {
                self.trigger_method = TriggerMethod::Keyboard;
                return self.trigger_method.clone();
            }
            if self.elapsed() >= self.max_wait { break; }
        }

        // 超时
        if self.media_event_count > 0 || self.video_element_found {
            self.trigger_method = TriggerMethod::Auto;
            self.last_error = Some("有播放活动但未检测到 playerCreated".into());
        } else if self.network_media_request_count > 0 {
            self.trigger_method = TriggerMethod::Auto;
            self.last_error = Some("检测到媒体请求但无播放事件".into());
        } else {
            self.last_error = Some("未能触发播放: 60s 内无播放器创建/播放事件/媒体请求".into());
        }
        self.trigger_method.clone()
    }

    pub fn on_media_created(&mut self) { self.player_created = true; }
    pub fn on_media_event(&mut self) { self.media_event_count += 1; }
    pub fn on_network_media(&mut self) { self.network_media_request_count += 1; }
    pub fn set_video_found(&mut self) { self.video_element_found = true; }

    pub fn elapsed(&self) -> Duration { self.start.elapsed() }

    pub fn trigger_method(&self) -> TriggerMethod { self.trigger_method.clone() }
    pub fn trigger_label(&self) -> &str {
        match self.trigger_method {
            TriggerMethod::Auto => "auto",
            TriggerMethod::Click => "click",
            TriggerMethod::Keyboard => "keyboard",
            TriggerMethod::None => "none",
        }
    }

    pub fn diagnostics(&self) -> VideoDiagnostics {
        VideoDiagnostics {
            player_created: self.player_created,
            media_event_count: self.media_event_count,
            network_media_request_count: self.network_media_request_count,
            video_element_found: self.video_element_found,
            trigger_method: self.trigger_label().to_string(),
            last_error: self.last_error.clone(),
            total_wait_ms: self.elapsed().as_millis() as u64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VideoDiagnostics {
    pub player_created: bool,
    pub media_event_count: u32,
    pub network_media_request_count: u32,
    pub video_element_found: bool,
    pub trigger_method: String,
    pub last_error: Option<String>,
    pub total_wait_ms: u64,
}
