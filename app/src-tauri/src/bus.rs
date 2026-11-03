//! Internal event bus. Modules publish and subscribe to [`AppEvent`]s; the
//! bus is also forwarded to the web front end as Tauri events. Later phases
//! add variants (hands_imported, tables_changed, play_mode_changed…).

use lps_config::ConfigEvent;
use serde::Serialize;
use tokio::sync::broadcast;

/// Everything that can travel on the bus.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "topic", content = "payload", rename_all = "snake_case")]
pub enum AppEvent {
    Config(ConfigEvent),
}

impl AppEvent {
    /// Name of the Tauri event the front end listens to.
    pub fn channel(&self) -> &'static str {
        match self {
            AppEvent::Config(_) => "config://event",
        }
    }
}

/// Cloneable handle to the bus.
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<AppEvent>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(256)
    }
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        Self { tx: broadcast::channel(capacity).0 }
    }

    /// Publishes an event; having no subscriber is not an error.
    pub fn publish(&self, e: AppEvent) {
        let _ = self.tx.send(e);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribers_receive_published_events() {
        let bus = EventBus::default();
        let mut rx = bus.subscribe();
        bus.publish(AppEvent::Config(ConfigEvent::Changed { family: "settings".into(), id: "settings".into() }));
        let e = rx.try_recv().unwrap();
        assert_eq!(e.channel(), "config://event");
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(json["topic"], "config");
        assert_eq!(json["payload"]["kind"], "changed");
    }
}
