use crate::engines::dns::DnsResult;
use crate::engines::http::HttpResult;

use super::events::VideoEvent;

#[derive(Debug, Clone, Default)]
pub struct VideoMetrics {
    pub platform: String,
    // 网络
    pub dns_time_ms: f64,
    pub dns_success: bool,
    pub tcp_time_ms: f64,
    pub http_response_ms: f64,
    // 播放器
    pub player_type: Option<String>,
    pub mime_type: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub resolution: Option<String>,
    pub fps: Option<f64>,
    pub video_bitrate_kbps: Option<f64>,
    pub audio_bitrate_kbps: Option<f64>,
    // 播放质量
    pub first_play_time_ms: Option<f64>,
    pub play_success: bool,
    pub buffer_count: u32,
    pub buffer_time_ms: f64,
    pub dropped_frames: u64,
    pub decoded_frames: u64,
    // 网络流量
    pub video_host: Option<String>,
    pub cdn_node: Option<String>,
    pub segment_count: u32,
    pub total_bytes: u64,
    pub download_speed: Option<f64>,
    pub peak_speed: Option<f64>,
    // 诊断
    pub trigger_method: String,
    pub video_color: Option<String>,
    pub page_title: Option<String>,
    pub screenshot: Option<Vec<u8>>,
    pub error: Option<String>,
    // 卡顿
    pub stutter_count: u32,
    pub stutter_duration_ms: f64,
    pub play_duration_sec: f64,
    pub stutter_ratio: f64,
    pub video_width: u32,
    pub video_height: u32,
    pub video_duration_sec: f64,
}

pub struct MetricCollector {
    pub metrics: VideoMetrics,
    pub engine_start: std::time::Instant,
    buffer_active: bool,
    buffer_start_time: std::time::Instant,
    play_started: bool,
    first_play_elapsed: Option<f64>,
    // 网络速度采样
    last_bytes_sample_time: std::time::Instant,
    current_sample_bytes: u64,
    peak_bps: f64,
    // 卡顿
    last_current_time: f64,
    stutter_active: bool,
    stutter_start_time: std::time::Instant,
}

impl MetricCollector {
    pub fn new(platform: &str, dns: &DnsResult, http: &HttpResult) -> Self {
        Self {
            metrics: VideoMetrics {
                platform: platform.to_string(),
                dns_time_ms: dns.dns_time_ms,
                dns_success: dns.dns_success,
                tcp_time_ms: http.tcp_time_ms,
                http_response_ms: http.ttfb_ms,
                trigger_method: "none".into(),
                ..Default::default()
            },
            engine_start: std::time::Instant::now(),
            buffer_active: false,
            buffer_start_time: std::time::Instant::now(),
            play_started: false,
            first_play_elapsed: None,
            last_bytes_sample_time: std::time::Instant::now(),
            current_sample_bytes: 0,
            peak_bps: 0.0,
            last_current_time: 0.0,
            stutter_active: false,
            stutter_start_time: std::time::Instant::now(),
        }
    }

    pub fn on_event(&mut self, event: &VideoEvent) {
        match event {
            VideoEvent::PlayStarted { player_id: _, video_src, meta: _ } => {
                if !self.play_started {
                    self.play_started = true;
                    self.first_play_elapsed = Some(self.engine_start.elapsed().as_secs_f64() * 1000.0);
                    self.metrics.play_success = true;
                    self.metrics.first_play_time_ms = self.first_play_elapsed;
                }
                if let Some(src) = video_src {
                    if self.metrics.video_host.is_none() {
                        if let Ok(parsed) = url::Url::parse(src) {
                            self.metrics.video_host = parsed.host_str().map(|s| s.to_string());
                        }
                    }
                }
            }
            VideoEvent::BufferStarted { .. } => {
                if !self.buffer_active {
                    self.buffer_active = true;
                    self.buffer_start_time = std::time::Instant::now();
                    self.metrics.buffer_count += 1;
                }
            }
            VideoEvent::BufferEnded { duration_ms, .. } => {
                if self.buffer_active {
                    self.buffer_active = false;
                    let dur = *duration_ms;
                    self.metrics.buffer_time_ms += if dur > 0.0 { dur } else {
                        self.buffer_start_time.elapsed().as_secs_f64() * 1000.0
                    };
                }
            }
            VideoEvent::ResolutionChanged { width, height, .. } => {
                self.metrics.resolution = Some(format!("{}x{}", width, height));
                if self.metrics.video_width == 0 { self.metrics.video_width = *width; }
                if self.metrics.video_height == 0 { self.metrics.video_height = *height; }
            }
            VideoEvent::BitrateChanged { video_kbps, audio_kbps, .. } => {
                self.metrics.video_bitrate_kbps = Some(*video_kbps);
                self.metrics.audio_bitrate_kbps = Some(*audio_kbps);
            }
            VideoEvent::DroppedFramesChanged { dropped, decoded, .. } => {
                self.metrics.dropped_frames = *dropped;
                self.metrics.decoded_frames = *decoded;
            }
            VideoEvent::FpsChanged { fps, .. } => {
                self.metrics.fps = Some(*fps);
            }
            VideoEvent::CodecDetected { video_codec, audio_codec, mime_type, .. } => {
                self.metrics.video_codec = Some(video_codec.clone());
                self.metrics.audio_codec = Some(audio_codec.clone());
                self.metrics.mime_type = Some(mime_type.clone());
            }
            VideoEvent::SegmentLoaded { url, host, size_bytes, .. } => {
                self.metrics.segment_count += 1;
                self.metrics.total_bytes += size_bytes;
                if self.metrics.video_host.is_none() {
                    self.metrics.video_host = Some(host.clone());
                }
                // 速度采样: 每秒计算一次
                self.current_sample_bytes += size_bytes;
                let now = std::time::Instant::now();
                let sample_elapsed = now.duration_since(self.last_bytes_sample_time);
                if sample_elapsed.as_secs_f64() >= 1.0 {
                    let bps = self.current_sample_bytes as f64 / sample_elapsed.as_secs_f64();
                    if bps > self.peak_bps { self.peak_bps = bps; }
                    self.current_sample_bytes = 0;
                    self.last_bytes_sample_time = now;
                }
            }
            VideoEvent::CdnDetected { cdn_node, .. } => {
                if self.metrics.cdn_node.is_none() {
                    self.metrics.cdn_node = Some(cdn_node.clone());
                }
            }
            VideoEvent::VideoError { error_type, message, .. } => {
                if self.metrics.error.is_none() {
                    self.metrics.error = Some(format!("{}: {}", error_type, message));
                }
            }
            VideoEvent::PlayEnded { .. } => {
                self.metrics.play_duration_sec = self.play_duration();
            }
            _ => {}
        }
    }

    pub fn update_trigger_method(&mut self, method: &str) {
        self.metrics.trigger_method = method.to_string();
    }

    pub fn set_page_title(&mut self, title: String) {
        self.metrics.page_title = Some(title);
    }

    pub fn set_screenshot(&mut self, data: Vec<u8>) {
        self.metrics.screenshot = Some(data);
    }

    pub fn set_error(&mut self, err: String) {
        self.metrics.error = Some(err);
    }

    fn play_duration(&self) -> f64 {
        if let Some(first) = self.first_play_elapsed {
            let now_ms = self.engine_start.elapsed().as_secs_f64() * 1000.0;
            ((now_ms - first) / 1000.0).max(0.0)
        } else {
            0.0
        }
    }

    /// 更新卡顿相关指标（从 JS 轮询数据）
    pub fn update_stutter(&mut self, current_time: f64, width: u32, height: u32, duration: f64) {
        if self.metrics.video_width == 0 { self.metrics.video_width = width; }
        if self.metrics.video_height == 0 { self.metrics.video_height = height; }
        if self.metrics.video_duration_sec == 0.0 { self.metrics.video_duration_sec = duration; }

        if self.play_started && self.last_current_time > 0.0 {
            let delta = (current_time - self.last_current_time).abs();
            if delta < 0.1 {
                if !self.stutter_active {
                    self.stutter_active = true;
                    self.stutter_start_time = std::time::Instant::now();
                    self.metrics.stutter_count += 1;
                }
            } else if self.stutter_active {
                self.stutter_active = false;
                self.metrics.stutter_duration_ms += self.stutter_start_time.elapsed().as_secs_f64() * 1000.0;
            }
        }
        self.last_current_time = current_time;
    }

    pub fn finalize(mut self) -> VideoMetrics {
        // 处理未结束的缓冲
        if self.buffer_active {
            self.metrics.buffer_time_ms += self.buffer_start_time.elapsed().as_secs_f64() * 1000.0;
        }
        // 处理未结束的卡顿
        if self.stutter_active {
            self.metrics.stutter_duration_ms += self.stutter_start_time.elapsed().as_secs_f64() * 1000.0;
        }
        // 下载速度
        let elapsed = self.engine_start.elapsed().as_secs_f64();
        if self.metrics.total_bytes > 0 && elapsed > 0.0 {
            self.metrics.download_speed = Some(self.metrics.total_bytes as f64 / elapsed / 1024.0);
        }
        if self.peak_bps > 0.0 {
            self.metrics.peak_speed = Some(self.peak_bps / 1024.0);
        }
        // play_duration
        self.metrics.play_duration_sec = self.play_duration();
        // stutter_ratio
        if self.metrics.play_duration_sec > 0.0 {
            self.metrics.stutter_ratio = self.metrics.stutter_duration_ms / 1000.0 / self.metrics.play_duration_sec * 100.0;
        }
        self.metrics
    }
}

impl VideoMetrics {
    pub fn error_result(platform: &str, dns: DnsResult, http: HttpResult, msg: &str) -> Self {
        Self {
            platform: platform.to_string(),
            dns_time_ms: dns.dns_time_ms,
            dns_success: dns.dns_success,
            tcp_time_ms: http.tcp_time_ms,
            http_response_ms: http.ttfb_ms,
            error: Some(msg.to_string()),
            ..Default::default()
        }
    }
}
