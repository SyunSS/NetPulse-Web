use std::time::Instant;

use chromiumoxide::cdp::browser_protocol::network::{
    EventDataReceived, EventRequestWillBeSent, EventResponseReceived,
};

use super::super::events::{EventMeta, VideoEvent};

pub struct NetworkCollector {
    tx: tokio::sync::mpsc::UnboundedSender<VideoEvent>,
}

impl NetworkCollector {
    pub fn new(tx: tokio::sync::mpsc::UnboundedSender<VideoEvent>) -> Self {
        Self { tx }
    }

    pub fn handle_request_will_be_sent(&self, event: EventRequestWillBeSent) {
        let url = &event.request.url;
        let lower = url.to_lowercase();

        let video_ext = lower.contains(".mp4") || lower.contains(".m3u8")
            || lower.contains(".mpd") || lower.contains(".m4s")
            || lower.contains(".ts") || lower.contains(".flv")
            || lower.contains(".webm") || lower.contains(".ogg");

        if video_ext {
            let _ = self.tx.send(VideoEvent::SegmentLoaded {
                url: url.clone(),
                host: String::new(),
                size_bytes: 0,
                meta: EventMeta::now(),
            });
        }
    }

    pub fn handle_response_received(&self, event: EventResponseReceived) {
        if let Some(ref remote_ip) = event.response.remote_ip_address {
            let _ = self.tx.send(VideoEvent::CdnDetected {
                host: String::new(),
                cdn_node: remote_ip.clone(),
                meta: EventMeta::now(),
            });
        }
    }

    pub fn handle_data_received(&self, event: EventDataReceived) {
        let bytes = event.data_length as u64;
        let _ = self.tx.send(VideoEvent::BytesReceived {
            bytes,
            meta: EventMeta::now(),
        });
    }
}
