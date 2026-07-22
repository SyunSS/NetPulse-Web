use chromiumoxide::cdp::browser_protocol::runtime::EventConsoleApiCalled;
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
        let console_type = event.type_.to_string();
        let text = event.args.iter()
            .find_map(|arg| arg.value.as_ref().map(|v| v.to_string()))
            .unwrap_or_default();

        debug!("Console [{}]: {}", console_type, text);

        if console_type == "error" {
            let _ = self.tx.send(VideoEvent::JsConsoleError {
                text,
                meta: EventMeta::now(),
            });
        }
    }
}
