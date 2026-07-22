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
        let text = event.args.iter()
            .find_map(|arg| arg.value.as_ref().map(|v| v.to_string()))
            .unwrap_or_default();

        debug!("Console [{:?}]: {}", event.r#type, text);

        if is_error {
            let _ = self.tx.send(VideoEvent::JsConsoleError {
                text,
                meta: EventMeta::now(),
            });
        }
    }
}
