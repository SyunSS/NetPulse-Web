use std::time::Instant;

use tracing::info;

use super::events::VideoEvent;

pub struct DiagnosticLogger {
    start: Instant,
}

impl DiagnosticLogger {
    pub fn new() -> Self {
        Self { start: Instant::now() }
    }

    pub fn log_phase(&self, phase: &str) {
        let elapsed = self.start.elapsed().as_secs_f64() * 1000.0;
        info!("[VideoEngine +{:.0}ms] {}", elapsed, phase);
    }

    pub fn log_event(&self, event: &VideoEvent) {
        let elapsed = self.start.elapsed().as_secs_f64() * 1000.0;
        match event {
            VideoEvent::ChromiumStarted { pid, .. } => {
                info!("[VideoEngine +{:.0}ms] Chromium 启动成功 (pid={})", elapsed, pid);
            }
            VideoEvent::CdpConnected { ws_url, .. } => {
                info!("[VideoEngine +{:.0}ms] CDP 连接成功, ws={}", elapsed, ws_url);
            }
            VideoEvent::PageLoaded { url, final_url, .. } => {
                info!("[VideoEngine +{:.0}ms] 页面加载完成: {} -> {:?}", elapsed, url, final_url);
            }
            VideoEvent::HooksInjected { hook_count, .. } => {
                info!("[VideoEngine +{:.0}ms] {} 个 JS Hook 注入完成", elapsed, hook_count);
            }
            VideoEvent::VideoElementDiscovered { selector, count, .. } => {
                info!("[VideoEngine +{:.0}ms] 发现 {} 个 video 元素 (selector={})", elapsed, count, selector);
            }
            VideoEvent::PlayerIdentified { platform, player_type, .. } => {
                info!("[VideoEngine +{:.0}ms] 播放器识别: platform={}, type={}", elapsed, platform, player_type);
            }
            VideoEvent::PlayStarted { player_id, video_src, .. } => {
                info!("[VideoEngine +{:.0}ms] 收到播放事件 player_id={:?}, src={:?}", elapsed, player_id, video_src);
            }
            VideoEvent::PlayPaused { player_id, .. } => {
                info!("[VideoEngine +{:.0}ms] 播放暂停 player_id={:?}", elapsed, player_id);
            }
            VideoEvent::PlayResumed { player_id, .. } => {
                info!("[VideoEngine +{:.0}ms] 播放恢复 player_id={:?}", elapsed, player_id);
            }
            VideoEvent::PlayEnded { player_id, .. } => {
                info!("[VideoEngine +{:.0}ms] 播放结束 player_id={:?}", elapsed, player_id);
            }
            VideoEvent::Seek { from_sec, to_sec, .. } => {
                info!("[VideoEngine +{:.0}ms] Seek: {:.1}s -> {:.1}s", elapsed, from_sec, to_sec);
            }
            VideoEvent::BufferStarted { player_id, .. } => {
                info!("[VideoEngine +{:.0}ms] 缓冲开始 player_id={:?}", elapsed, player_id);
            }
            VideoEvent::BufferEnded { player_id, duration_ms, .. } => {
                info!("[VideoEngine +{:.0}ms] 缓冲结束 player_id={:?}, duration={}ms", elapsed, player_id, duration_ms);
            }
            VideoEvent::ResolutionChanged { width, height, .. } => {
                info!("[VideoEngine +{:.0}ms] 分辨率变化: {}x{}", elapsed, width, height);
            }
            VideoEvent::BitrateChanged { video_kbps, audio_kbps, .. } => {
                info!("[VideoEngine +{:.0}ms] 码率变化: video={}kbps, audio={}kbps", elapsed, video_kbps, audio_kbps);
            }
            VideoEvent::DroppedFramesChanged { dropped, decoded, .. } => {
                info!("[VideoEngine +{:.0}ms] 丢帧: dropped={}, decoded={}", elapsed, dropped, decoded);
            }
            VideoEvent::FpsChanged { fps, .. } => {
                info!("[VideoEngine +{:.0}ms] FPS: {:.1}", elapsed, fps);
            }
            VideoEvent::CodecDetected { video_codec, audio_codec, mime_type, .. } => {
                info!("[VideoEngine +{:.0}ms] 编码: video={}, audio={}, mime={}", elapsed, video_codec, audio_codec, mime_type);
            }
            VideoEvent::SegmentLoaded { url: _, host, size_bytes, .. } => {
                info!("[VideoEngine +{:.0}ms] 分片加载: host={}, size={}B", elapsed, host, size_bytes);
            }
            VideoEvent::CdnDetected { host, cdn_node, .. } => {
                info!("[VideoEngine +{:.0}ms] CDN 节点: host={}, cdn={}", elapsed, host, cdn_node);
            }
            VideoEvent::BytesReceived { bytes, .. } => {
                info!("[VideoEngine +{:.0}ms] 收到 {} bytes", elapsed, bytes);
            }
            VideoEvent::VideoError { error_type, message, .. } => {
                info!("[VideoEngine +{:.0}ms] 视频错误: type={}, msg={}", elapsed, error_type, message);
            }
            VideoEvent::JsConsoleError { text, .. } => {
                info!("[VideoEngine +{:.0}ms] JS Console Error: {}", elapsed, text);
            }
            VideoEvent::MetricsFinalized { play_detected, first_play_time_ms, total_buffer_count, total_buffer_time_ms, .. } => {
                info!("[VideoEngine +{:.0}ms] 最终指标: play={}, first_play={}ms, buffer_count={}, buffer_time={}ms",
                    elapsed, play_detected, first_play_time_ms, total_buffer_count, total_buffer_time_ms);
            }
        }
    }
}
