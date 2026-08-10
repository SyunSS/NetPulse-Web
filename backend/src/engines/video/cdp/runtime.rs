use chromiumoxide::cdp::js_protocol::runtime::EventConsoleApiCalled;
use tracing::debug;

use super::super::events::{EventMeta, VideoEvent};

pub struct RuntimeCollector {
    tx: tokio::sync::mpsc::UnboundedSender<VideoEvent>,
}

impl RuntimeCollector {
    pub fn new(tx: tokio::sync::mpsc::UnboundedSender<VideoEvent>) -> Self {
        Self { tx }
    }

    pub fn handle_console_api_called(&self, event: EventConsoleApiCalled) {
        let is_error = matches!(
            event.r#type,
            chromiumoxide::cdp::js_protocol::runtime::ConsoleApiCalledType::Error
        );
        let raw_text = event
            .args
            .iter()
            .find_map(|arg| arg.value.as_ref().map(|v| v.to_string()))
            .unwrap_or_default();
        let text = serde_json::from_str::<String>(&raw_text).unwrap_or(raw_text);

        debug!("Console [{:?}]: {}", event.r#type, text);

        if self.handle_np_hook(&text) {
            return;
        }

        if is_error {
            let _ = self.tx.send(VideoEvent::JsConsoleError {
                text,
                meta: EventMeta::now(),
            });
        }
    }

    fn handle_np_hook(&self, text: &str) -> bool {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
            return false;
        };
        if value.get("type").and_then(|v| v.as_str()) != Some("np-hook") {
            return false;
        }

        let event = value.get("event").and_then(|v| v.as_str()).unwrap_or_default();
        match event {
            "playing_event" => {
                let src = value
                    .get("src")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
                let _ = self.tx.send(VideoEvent::PlayStarted {
                    player_id: None,
                    video_src: src,
                    meta: EventMeta::now(),
                });
            }
            "pause" | "pause_event" => {
                let _ = self.tx.send(VideoEvent::PlayPaused {
                    player_id: None,
                    meta: EventMeta::now(),
                });
            }
            "waiting_event" => {
                let _ = self.tx.send(VideoEvent::BufferStarted {
                    player_id: None,
                    meta: EventMeta::now(),
                });
            }
            "canplay_event" => {
                let _ = self.tx.send(VideoEvent::BufferEnded {
                    player_id: None,
                    duration_ms: 0.0,
                    meta: EventMeta::now(),
                });
            }
            "ended_event" => {
                let _ = self.tx.send(VideoEvent::PlayEnded {
                    player_id: None,
                    meta: EventMeta::now(),
                });
            }
            "error_event" => {
                let message = value
                    .get("msg")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown media error")
                    .to_string();
                let _ = self.tx.send(VideoEvent::VideoError {
                    error_type: "media_element".into(),
                    message,
                    meta: EventMeta::now(),
                });
            }
            "mse_addSourceBuffer" => {
                if let Some(mime) = value.get("mime").and_then(|v| v.as_str()) {
                    let _ = self.tx.send(VideoEvent::CodecDetected {
                        video_codec: String::new(),
                        audio_codec: String::new(),
                        mime_type: mime.to_string(),
                        meta: EventMeta::now(),
                    });
                }
            }
            "mse_appendBuffer" => {}
            "fetch" | "xhr" => {
                let url = value.get("url").and_then(|v| v.as_str()).unwrap_or_default();
                if is_media_url(url) {
                    let host = url::Url::parse(url)
                        .ok()
                        .and_then(|parsed| parsed.host_str().map(str::to_string))
                        .unwrap_or_default();
                    let _ = self.tx.send(VideoEvent::SegmentLoaded {
                        url: url.to_string(),
                        host,
                        size_bytes: 0,
                        meta: EventMeta::now(),
                    });
                }
            }
            "mutation_video_added" => {
                let _ = self.tx.send(VideoEvent::VideoElementDiscovered {
                    selector: "video".into(),
                    count: 1,
                    meta: EventMeta::now(),
                });
            }
            _ => {}
        }

        true
    }
}

fn is_media_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.contains(".mp4")
        || lower.contains(".m3u8")
        || lower.contains(".mpd")
        || lower.contains(".m4s")
        || lower.contains(".ts")
        || lower.contains(".flv")
        || lower.contains(".webm")
        || lower.contains("videoplayback")
        || lower.contains("mime=video")
}
