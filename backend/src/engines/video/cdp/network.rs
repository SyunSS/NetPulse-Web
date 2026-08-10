use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use chromiumoxide::cdp::browser_protocol::network::{
    EventDataReceived, EventRequestWillBeSent, EventResponseReceived,
};

use super::super::events::{EventMeta, VideoEvent};

#[derive(Clone)]
pub struct NetworkCollector {
    tx: tokio::sync::mpsc::UnboundedSender<VideoEvent>,
    media_requests: Arc<Mutex<HashSet<String>>>,
    reported_segments: Arc<Mutex<HashSet<String>>>,
}

impl NetworkCollector {
    pub fn new(tx: tokio::sync::mpsc::UnboundedSender<VideoEvent>) -> Self {
        Self {
            tx,
            media_requests: Arc::new(Mutex::new(HashSet::new())),
            reported_segments: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn handle_request_will_be_sent(&self, event: EventRequestWillBeSent) {
        let url = &event.request.url;
        if is_media_url(url) {
            if let Ok(mut requests) = self.media_requests.lock() {
                requests.insert(event.request_id.as_ref().to_string());
            }
            if let Ok(mut reported) = self.reported_segments.lock() {
                reported.insert(event.request_id.as_ref().to_string());
            }
            let host = url::Url::parse(url)
                .ok()
                .and_then(|parsed| parsed.host_str().map(str::to_string))
                .unwrap_or_default();
            let _ = self.tx.send(VideoEvent::SegmentLoaded {
                url: url.clone(),
                host,
                size_bytes: 0,
                meta: EventMeta::now(),
            });
        }
    }

    pub fn handle_response_received(&self, event: EventResponseReceived) {
        let url = &event.response.url;
        let mime = event.response.mime_type.to_lowercase();
        let is_media = is_media_url(url)
            || mime.starts_with("video/")
            || mime.contains("mpegurl")
            || mime.contains("dash+xml")
            || mime.contains("mp4");
        if is_media {
            if let Ok(mut requests) = self.media_requests.lock() {
                requests.insert(event.request_id.as_ref().to_string());
            }
            let request_id = event.request_id.as_ref().to_string();
            let should_report = self
                .reported_segments
                .lock()
                .map(|mut reported| reported.insert(request_id))
                .unwrap_or(false);
            if should_report {
                let host = url::Url::parse(url)
                    .ok()
                    .and_then(|parsed| parsed.host_str().map(str::to_string))
                    .unwrap_or_default();
                let _ = self.tx.send(VideoEvent::SegmentLoaded {
                    url: url.clone(),
                    host,
                    size_bytes: 0,
                    meta: EventMeta::now(),
                });
            }
        }

        if is_media {
            if let Some(ref remote_ip) = event.response.remote_ip_address {
                let _ = self.tx.send(VideoEvent::CdnDetected {
                    host: url::Url::parse(url)
                        .ok()
                        .and_then(|parsed| parsed.host_str().map(str::to_string))
                        .unwrap_or_default(),
                    cdn_node: remote_ip.clone(),
                    meta: EventMeta::now(),
                });
            }
        }
    }

    pub fn handle_data_received(&self, event: EventDataReceived) {
        let request_id = event.request_id.as_ref().to_string();
        let is_media = self
            .media_requests
            .lock()
            .map(|requests| requests.contains(&request_id))
            .unwrap_or(false);
        if !is_media {
            return;
        }

        let bytes = event.encoded_data_length.max(event.data_length) as u64;
        let _ = self.tx.send(VideoEvent::BytesReceived {
            bytes,
            meta: EventMeta::now(),
        });
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
        || lower.contains(".ogg")
        || lower.contains("videoplayback")
        || lower.contains("mime=video")
}
