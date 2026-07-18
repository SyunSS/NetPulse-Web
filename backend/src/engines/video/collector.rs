use std::sync::Mutex;

use crate::engines::browser::provider::CdpEvent;

// ─── MediaMetrics ────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct MediaMetrics {
    pub player_type: Option<String>,
    pub mime_type: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub resolution: Option<String>,
    pub fps: Option<f64>,
    pub video_bitrate_kbps: Option<f64>,
    pub audio_bitrate_kbps: Option<f64>,
    pub first_play_time_ms: Option<f64>,
    pub play_success: bool,
    pub buffer_count: i32,
    pub buffer_time_ms: f64,
    pub dropped_frames: i32,
    pub decoded_frames: i32,
    pub player_id: Option<String>,
}

// ─── NetworkMetrics ──────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct NetworkMetrics {
    pub video_host: Option<String>,
    pub audio_host: Option<String>,
    pub cdn_node: Option<String>,
    pub segment_count: i32,
    pub total_bytes: u64,
    pub download_speed: Option<f64>,
    pub avg_speed: Option<f64>,
    pub peak_speed: Option<f64>,
    pub speed_samples: Vec<f64>,
}

// ─── MediaCollector ──────────────────────────────────

pub struct MediaCollector {
    data: Mutex<MediaMetrics>,
    started_at: std::time::Instant,
}

impl MediaCollector {
    pub fn new() -> Self {
        Self { data: Mutex::new(MediaMetrics::default()), started_at: std::time::Instant::now() }
    }

    pub fn handle_cdp(&self, event: &CdpEvent) {
        if let Ok(mut m) = self.data.lock() {
            match event.method.as_str() {
                "Media.playerCreated" => {
                    m.player_id = event.params.get("playerId").and_then(|v| v.as_str().map(|s| s.to_string()));
                }
                "Media.playerPropertiesChanged" => {
                    if let Some(props) = event.params.get("properties").and_then(|v| v.as_array()) {
                        for p in props {
                            let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
                            let value = p.get("value").and_then(|v| v.as_str()).unwrap_or("");
                            match name {
                                "kPlaybackStateInfo" => m.player_type = Some(value.to_string()),
                                "kMimeType" => {
                                    m.mime_type = Some(value.to_string());
                                    parse_codecs(value, &mut *m);
                                }
                                "kResolution" => {
                                    let parts: Vec<&str> = value.split('x').collect();
                                    if parts.len() >= 2 {
                                        m.resolution = Some(value.to_string());
                                    }
                                }
                                "kFps" => m.fps = value.parse::<f64>().ok(),
                                "kDroppedFrames" => m.dropped_frames = value.parse::<i32>().unwrap_or(0),
                                "kDecodedFrames" => m.decoded_frames = value.parse::<i32>().unwrap_or(0),
                                "kAudioBitrateKbps" => m.audio_bitrate_kbps = value.parse::<f64>().ok(),
                                "kVideoBitrateKbps" => m.video_bitrate_kbps = value.parse::<f64>().ok(),
                                _ => {}
                            }
                        }
                    }
                }
                "Media.playerEventsAdded" => {
                    if let Some(events) = event.params.get("events").and_then(|v| v.as_array()) {
                        for e in events {
                            let ev = e.get("value").and_then(|v| v.as_str()).unwrap_or("");
                            let ts = e.get("timestamp").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            match ev {
                                "playing" => {
                                    if m.first_play_time_ms.is_none() {
                                        m.first_play_time_ms = Some(ts);
                                        m.play_success = true;
                                    }
                                }
                                "buffer_start" | "buffering" => {
                                    m.buffer_count += 1;
                                    m.buffer_time_ms += ts;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn snapshot(&self) -> MediaMetrics {
        self.data.lock().map(|m| m.clone()).unwrap_or_default()
    }
}

fn parse_codecs(value: &str, m: &mut MediaMetrics) {
    // 解析 "video/mp4; codecs=\"av01.0.05M.08, mp4a.40.2\""
    let parts: Vec<&str> = value.split("codecs=").collect();
    if parts.len() < 2 { return; }
    let codecs_str = parts[1].trim_matches('"').trim_matches('\'');
    for part in codecs_str.split(',') {
        let c = part.trim();
        if c.starts_with("av") || c == "h264" || c == "h265" || c == "vp9" || c == "vp8" {
            m.video_codec = Some(c.to_string());
        } else if c.starts_with("mp4a") || c == "aac" || c == "opus" {
            m.audio_codec = Some(c.to_string());
        }
    }
}

// ─── NetworkCollector ────────────────────────────────

pub struct NetworkCollector {
    data: Mutex<NetworkData>,
    started_at: std::time::Instant,
}

#[derive(Debug, Default)]
struct NetworkData {
    video_host: Option<String>,
    audio_host: Option<String>,
    cdn_node: Option<String>,
    segment_count: i32,
    total_bytes: u64,
    speed_samples: Vec<f64>,
    last_report: Option<std::time::Instant>,
    segment_bytes: u64,
    requests: std::collections::HashMap<String, String>, // requestId → url
}

impl NetworkCollector {
    pub fn new() -> Self {
        Self { data: Mutex::new(NetworkData::default()), started_at: std::time::Instant::now() }
    }

    pub fn handle_cdp(&self, event: &CdpEvent) {
        if let Ok(mut n) = self.data.lock() {
            match event.method.as_str() {
                "Network.requestWillBeSent" => {
                    let url = event.params.get("url").and_then(|v| v.as_str()).unwrap_or("");
                    let req_id = event.params.get("requestId").and_then(|v| v.as_str()).unwrap_or("");
                    let rtype = event.params.get("type").and_then(|v| v.as_str()).unwrap_or("");

                    n.requests.insert(req_id.to_string(), url.to_string());

                    // 识别视频/音频资源
                    if is_video_url(url) || rtype == "Media" {
                        if n.video_host.is_none() {
                            n.video_host = extract_host(url);
                        }
                        n.segment_count += 1;
                    }
                    if rtype == "Media" && url.contains("audio") && n.audio_host.is_none() {
                        n.audio_host = extract_host(url);
                    }
                }
                "Network.responseReceived" => {
                    let req_id = event.params.get("requestId").and_then(|v| v.as_str()).unwrap_or("");
                    if let Some(resp) = event.params.get("response") {
                        if n.cdn_node.is_none() {
                            n.cdn_node = resp.get("remoteIPAddress")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                        }
                    }
                }
                "Network.dataReceived" => {
                    let data_len = event.params.get("dataLength").and_then(|v| v.as_u64()).unwrap_or(0);
                    if data_len > 0 {
                        n.total_bytes += data_len;
                        n.segment_bytes += data_len;

                        let now = std::time::Instant::now();
                        if let Some(last) = n.last_report {
                            let elapsed = now.duration_since(last).as_secs_f64();
                            if elapsed >= 1.0 {
                                let speed = n.segment_bytes as f64 / elapsed / 1024.0;
                                n.speed_samples.push(speed);
                                n.segment_bytes = 0;
                                n.last_report = Some(now);
                            }
                        } else {
                            n.last_report = Some(now);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn snapshot(&self) -> NetworkMetrics {
        let n = self.data.lock().unwrap();
        let elapsed = self.started_at.elapsed().as_secs_f64();
        let avg = if elapsed > 0.0 && n.total_bytes > 0 {
            Some(n.total_bytes as f64 / elapsed / 1024.0)
        } else { None };
        let peak = n.speed_samples.iter().cloned().fold(0.0f64, f64::max);
        let current = n.speed_samples.last().copied();

        NetworkMetrics {
            video_host: n.video_host.clone(),
            audio_host: n.audio_host.clone(),
            cdn_node: n.cdn_node.clone(),
            segment_count: n.segment_count,
            total_bytes: n.total_bytes,
            download_speed: current,
            avg_speed: avg,
            peak_speed: if peak > 0.0 { Some(peak) } else { None },
            speed_samples: n.speed_samples.clone(),
        }
    }
}

fn is_video_url(url: &str) -> bool {
    let vid_exts = [".mp4", ".m3u8", ".mpd", ".m4s", ".ts", ".flv", ".webm"];
    vid_exts.iter().any(|ext| url.contains(ext))
}

fn extract_host(url: &str) -> Option<String> {
    url.split("://").nth(1)
        .and_then(|s| s.split('/').next())
        .and_then(|s| s.split(':').next())
        .map(|s| s.to_string())
}
