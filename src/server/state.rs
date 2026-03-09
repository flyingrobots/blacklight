use serde::{Serialize, Deserialize};
use ts_rs::TS;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::config::BlacklightConfig;
pub use crate::db::DbPool;
use crate::enrich::{EnrichProgress, EnrichReport};
use crate::indexer::{IndexProgress, IndexReport};
use crate::notifications::NotificationSender;

/// Indexer run status.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub enum IndexerStatus {
    #[default]
    Idle,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// Migration run status.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub enum MigrationStatus {
    #[default]
    Idle,
    Running,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct MigrationProgress {
    pub total_sessions: usize,
    pub backed_up: usize,
    pub fingerprints_updated: usize,
}

/// Shared migration state accessible from API handlers.
pub struct MigrationState {
    pub status: MigrationStatus,
    pub progress: Arc<Mutex<MigrationProgress>>,
    pub error_message: Option<String>,
}

impl Default for MigrationState {
    fn default() -> Self {
        Self {
            status: MigrationStatus::Idle,
            progress: Arc::new(Mutex::new(MigrationProgress::default())),
            error_message: None,
        }
    }
}

/// Enricher run status.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub enum EnricherStatus {
    #[default]
    Idle,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Shared enricher state accessible from API handlers.
pub struct EnricherState {
    pub status: EnricherStatus,
    pub cancel_flag: Arc<AtomicBool>,
    pub progress: Arc<EnrichProgress>,
    pub latest_report: Option<EnrichReport>,
    pub error_message: Option<String>,
    pub log_lines: Arc<Mutex<Vec<String>>>,
}

impl Default for EnricherState {
    fn default() -> Self {
        Self {
            status: EnricherStatus::Idle,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(EnrichProgress::default()),
            latest_report: None,
            error_message: None,
            log_lines: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl EnricherState {
    /// Reset for a new run: clear counters, reset cancel flag, set Running.
    pub fn reset_for_run(&mut self) {
        self.status = EnricherStatus::Running;
        self.cancel_flag.store(false, Ordering::Relaxed);
        self.progress = Arc::new(EnrichProgress::default());
        self.latest_report = None;
        self.error_message = None;
        self.log_lines.lock().unwrap().clear();
    }
}

/// Handle to a running scheduler task with cancellation support.
pub struct SchedulerHandle {
    pub cancel_flag: Arc<AtomicBool>,
}

impl SchedulerHandle {
    pub fn reset_for_run(&mut self) {
        self.cancel_flag.store(false, Ordering::Relaxed);
    }
}

/// Indexer control commands.
#[derive(Debug)]
pub enum IndexerCommand {
    Start { full: bool },
    Stop,
    Pause,
    Resume,
}

/// Shared indexer state broadcast to API handlers.
#[derive(Clone, Debug, Default, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct IndexerState {
    pub status: IndexerStatus,
    pub progress: IndexProgress,
    pub latest_report: Option<IndexReport>,
    pub error_message: Option<String>,
    pub run_id: Option<i64>,
}

/// Classifier control commands.
#[derive(Debug)]
pub enum ClassifierCommand {
    Start { limit: Option<usize>, force: bool },
    Stop,
}

/// Shared classifier state broadcast to API handlers.
#[derive(Clone, Debug, Default, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct ClassifierState {
    pub status: EnricherStatus, // reuse EnricherStatus
    pub sessions_total: usize,
    pub sessions_done: usize,
    pub sessions_failed: usize,
    pub latest_report: Option<crate::classifier::ClassifierReport>,
    pub error_message: Option<String>,
    pub run_id: Option<i64>,
    pub outdated_count: i64,
}

/// Shared application state passed to all axum handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DbPool>,
    pub config: Arc<BlacklightConfig>,
    pub indexer: tokio::sync::watch::Receiver<IndexerState>,
    pub indexer_tx: tokio::sync::mpsc::Sender<IndexerCommand>,
    pub classifier: tokio::sync::watch::Receiver<ClassifierState>,
    pub classifier_tx: tokio::sync::mpsc::Sender<ClassifierCommand>,
    pub enricher: Arc<tokio::sync::Mutex<EnricherState>>,
    pub migration: Arc<tokio::sync::Mutex<MigrationState>>,
    pub scheduler: Arc<tokio::sync::Mutex<Option<SchedulerHandle>>>,
    pub notifications: NotificationSender,
}
