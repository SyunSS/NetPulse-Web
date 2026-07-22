use chromiumoxide::cdp::browser_protocol::media::{
    EventPlayerCreated, EventPlayerEventsAdded, EventPlayerPropertiesChanged,
};
use tracing::{debug, info};

use super::super::events::{EventMeta, VideoEvent};

pub struct MediaCollector {
    tx: tokio::sync::mpsc::UnboundedSender<VideoEvent>,
}

impl MediaCollector {
    pub fn new(tx: tokio::sync::mpsc::UnboundedSender<VideoEvent>) -> Self {
        Self { tx }
    }

    pub fn handle_player_created(&self, event: EventPlayerCreated) {
        let player_id = event.player_id;
        let _ = self.tx.send(VideoEvent::PlayerIdentified {
            platform: "unknown".into(),
            player_type: format!("player_{}", player_id),
            meta: EventMeta::now(),
        });
    }

    pub fn handle_properties_changed(&self, event: EventPlayerPropertiesChanged) {
        for prop in event.properties.iter() {
            match prop.name.as_str() {
                "kResolution" => {
                    if let Some(res) = &prop.value {
                        let parts: Vec<&str> = res.split('x').collect();
                        if parts.len() == 2 {
                            if let (Ok(w), Ok(h)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                                let _ = self.tx.send(VideoEvent::ResolutionChanged {
                                    width: w, height: h, meta: EventMeta::now(),
                                });
                            }
                        }
                    }
                }
                "kFps" => {
                    if let Some(val) = &prop.value {
                        if let Ok(fps) = val.parse::<f64>() {
                            let _ = self.tx.send(VideoEvent::FpsChanged {
                                fps, meta: EventMeta::now(),
                            });
                        }
                    }
                }
                "kVideoBitrateKbps" => {
                    if let Some(val) = &prop.value {
                        if let Ok(vbr) = val.parse::<f64>() {
                            let _ = self.tx.send(VideoEvent::BitrateChanged {
                                video_kbps: vbr, audio_kbps: 0.0, meta: EventMeta::now(),
                            });
                        }
                    }
                }
                "kAudioBitrateKbps" => {
                    if let Some(val) = &prop.value {
                        if let Ok(abr) = val.parse::<f64>() {
                            let _ = self.tx.send(VideoEvent::BitrateChanged {
                                video_kbps: 0.0, audio_kbps: abr, meta: EventMeta::now(),
                            });
                        }
                    }
                }
                "kDroppedFrames" => {
                    if let Some(val) = &prop.value {
                        if let Ok(dropped) = val.parse::<u64>() {
                            let _ = self.tx.send(VideoEvent::DroppedFramesChanged {
                                dropped, decoded: 0, meta: EventMeta::now(),
                            });
                        }
                    }
                }
                "kDecodedFrames" => {
                    if let Some(val) = &prop.value {
                        if let Ok(decoded) = val.parse::<u64>() {
                            let _ = self.tx.send(VideoEvent::DroppedFramesChanged {
                                dropped: 0, decoded, meta: EventMeta::now(),
                            });
                        }
                    }
                }
                "kMimeType" => {
                    if let Some(mime) = &prop.value {
                        let (vc, ac) = parse_codecs(mime);
                        let _ = self.tx.send(VideoEvent::CodecDetected {
                            video_codec: vc,
                            audio_codec: ac,
                            mime_type: mime.clone(),
                            meta: EventMeta::now(),
                        });
                    }
                }
                _ => {
                    debug!(
                        "Media property: {} = {:?}",
                        prop.name, prop.value
                    );
                }
            }
        }
    }

    pub fn handle_events_added(&self, event: EventPlayerEventsAdded) {
        for ev in event.events.iter() {
            match ev.event_type.as_str() {
                "playing" => {
                    let _ = self.tx.send(VideoEvent::PlayStarted {
                        player_id: Some(event.player_id.clone()),
                        video_src: None,
                        meta: EventMeta::now(),
                    });
                }
                "buffering" | "buffer_start" => {
                    let _ = self.tx.send(VideoEvent::BufferStarted {
                        player_id: Some(event.player_id.clone()),
                        meta: EventMeta::now(),
                    });
                }
                "buffer_end" | "buffered" => {
                    let _ = self.tx.send(VideoEvent::BufferEnded {
                        player_id: Some(event.player_id.clone()),
                        duration_ms: 0.0,
                        meta: EventMeta::now(),
                    });
                }
                "error" => {
                    let error_msg = ev.value.clone().unwrap_or_default();
                    let _ = self.tx.send(VideoEvent::VideoError {
                        error_type: "media".into(),
                        message: format!("player_id={}, value={}", event.player_id, error_msg),
                        meta: EventMeta::now(),
                    });
                }
                "ended" => {
                    let _ = self.tx.send(VideoEvent::PlayEnded {
                        player_id: Some(event.player_id.clone()),
                        meta: EventMeta::now(),
                    });
                }
                "seek" => {
                    let _ = self.tx.send(VideoEvent::Seek {
                        from_sec: 0.0, to_sec: 0.0, meta: EventMeta::now(),
                    });
                }
                "pause" => {
                    let _ = self.tx.send(VideoEvent::PlayPaused {
                        player_id: Some(event.player_id.clone()),
                        meta: EventMeta::now(),
                    });
                }
                "resume" | "playing" => {}
                other => {
                    info!("Media event: {} (player={})", other, event.player_id);
                }
            }
        }
    }
}

fn parse_codecs(mime: &str) -> (String, String) {
    let mut vc = String::new();
    let mut ac = String::new();
    let lower = mime.to_lowercase();
    // 查找 codecs="..." 部分
    if let Some(start) = lower.find("codecs=") {
        let rest = &lower[start + 7..];
        let codec_str = rest.trim_matches('"');
        for part in codec_str.split(',') {
            let p = part.trim();
            if p.starts_with("av") || p.starts_with("h264") || p.starts_with("h265")
                || p.starts_with("vp9") || p.starts_with("vp8") || p.starts_with("hevc")
            {
                vc = p.to_string();
            } else if p.starts_with("mp4a") || p.starts_with("aac") || p.starts_with("opus")
                || p.starts_with("vorbis")
            {
                ac = p.to_string();
            }
        }
    }
    (vc, ac)
}
