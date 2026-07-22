use chromiumoxide::cdp::browser_protocol::page::EventLoadEventFired;
use tracing::info;

pub struct PageCollector;

impl PageCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn handle_load_event_fired(&self, _event: EventLoadEventFired) {
        info!("页面 Load 事件触发");
    }
}
