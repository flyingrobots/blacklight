use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::enrich::EnrichReport;
use crate::indexer::{IndexProgress, IndexReport};
use crate::notifications::NotificationSender;

/// A pool of SQLite connections for use with tokio::spawn_blocking.
/// rusqlite Connection is !Send, so we hold them behind a Mutex and
/// move them into blocking tasks.
pub struct DbPool {
    connections: Mutex<Vec<Connection>>,
    db_path: PathBuf,
    pool_size: usize,
}

impl DbPool {
    pub fn new(db_path: &Path, pool_size: usize) -> Result<Self> {
        let mut connections = Vec::with_capacity(pool_size);
        for _ in 0..pool_size {
            connections.push(crate::db::open(db_path)?);
        }
        Ok(Self {
            connections: Mutex::new(connections),
            db_path: db_path.to_path_buf(),
            pool_size,
        })
    }

    /// Execute a closure with a connection from the pool inside spawn_blocking.
    pub async fn call<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let conn = self.checkout()?;
        let result = tokio::task::spawn_blocking(move || f(&conn))
            .await
            .context("spawn_blocking join error")?;
        // Note: connection is dropped after spawn_blocking completes.
        // For a production pool we'd return it, but opening is cheap with WAL.
        result
    }

    fn checkout(&self) -> Result<Connection> {
        let mut pool = self.connections.lock().unwrap();
        if let Some(conn) = pool.pop() {
            Ok(conn)
        } else {
            // Pool exhausted — open a new connection
            crate::db::open(&self.db_path)
        }
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    #[allow(dead_code)]
    pub fn pool_size(&self) -> usize {
        self.pool_size
    }
}

/// Indexer run status.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexerStatus {
    #[default]
    Idle,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// Shared indexer state accessible from API handlers.
pub struct IndexerState {
    pub status: IndexerStatus,
    pub progress: Arc<Mutex<IndexProgress>>,
    pub cancel_flag: Arc<AtomicBool>,
    pub pause_flag: Arc<AtomicBool>,
    pub latest_report: Option<IndexReport>,
    pub error_message: Option<String>,
}

impl Default for IndexerState {
    fn default() -> Self {
        Self {
            status: IndexerStatus::Idle,
            progress: Arc::new(Mutex::new(IndexProgress::default())),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            pause_flag: Arc::new(AtomicBool::new(false)),
            latest_report: None,
            error_message: None,
        }
    }
}

impl IndexerState {
    /// Reset for a new run: clear progress, reset cancel flag, set Running.
    pub fn reset_for_run(&mut self) {
        self.status = IndexerStatus::Running;
        self.cancel_flag.store(false, Ordering::Relaxed);
        self.pause_flag.store(false, Ordering::Relaxed);
        *self.progress.lock().unwrap() = IndexProgress::default();
        self.latest_report = None;
        self.error_message = None;
    }
}

/// Enricher run status.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
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
    pub sessions_total: usize,
    pub sessions_done: usize,
    pub sessions_failed: usize,
    pub latest_report: Option<EnrichReport>,
    pub error_message: Option<String>,
}

impl Default for EnricherState {
    fn default() -> Self {
        Self {
            status: EnricherStatus::Idle,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            sessions_total: 0,
            sessions_done: 0,
            sessions_failed: 0,
            latest_report: None,
            error_message: None,
        }
    }
}

impl EnricherState {
    /// Reset for a new run: clear counters, reset cancel flag, set Running.
    pub fn reset_for_run(&mut self) {
        self.status = EnricherStatus::Running;
        self.cancel_flag.store(false, Ordering::Relaxed);
        self.sessions_total = 0;
        self.sessions_done = 0;
        self.sessions_failed = 0;
        self.latest_report = None;
        self.error_message = None;
    }
}

/// Handle to a running scheduler task with cancellation support.
pub struct SchedulerHandle {
    pub cancel_flag: Arc<AtomicBool>,
}

/// Shared application state passed to all axum handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DbPool>,
    pub source_dir: PathBuf,
    pub indexer: Arc<tokio::sync::Mutex<IndexerState>>,
    pub enricher: Arc<tokio::sync::Mutex<EnricherState>>,
    pub scheduler: Arc<tokio::sync::Mutex<Option<SchedulerHandle>>>,
    pub notifications: NotificationSender,
}
