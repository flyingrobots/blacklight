use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationLevel {
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug, Serialize)]
pub struct Notification {
    pub level: NotificationLevel,
    pub message: String,
    pub timestamp_ms: u64,
}

pub type NotificationSender = broadcast::Sender<Notification>;

pub fn create_channel() -> NotificationSender {
    let (tx, _) = broadcast::channel(256);
    tx
}

pub fn notify(tx: &NotificationSender, level: NotificationLevel, message: impl Into<String>) {
    let notification = Notification {
        level,
        message: message.into(),
        timestamp_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
    };
    // Ignore "no receivers" error
    let _ = tx.send(notification);
}
