use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use tracing::{error, info, warn};

use crate::indexer::{IndexConfig, IndexProgress};
use crate::notifications::{self, NotificationLevel};
use crate::server::state::{AppState, IndexerCommand, IndexerState, IndexerStatus};

pub struct IndexerActor {
    state_tx: watch::Sender<IndexerState>,
    command_rx: mpsc::Receiver<IndexerCommand>,
    app_state: AppState,
    cancel_flag: Arc<AtomicBool>,
    pause_flag: Arc<AtomicBool>,
}

impl IndexerActor {
    pub fn spawn(
        app_state: AppState,
        command_rx: mpsc::Receiver<IndexerCommand>,
        state_tx: watch::Sender<IndexerState>,
    ) {
        let actor = Self {
            state_tx,
            command_rx,
            app_state,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            pause_flag: Arc::new(AtomicBool::new(false)),
        };

        tokio::spawn(async move {
            actor.run().await;
        });
    }

    async fn run(mut self) {
        info!("Indexer actor started");

        while let Some(cmd) = self.command_rx.recv().await {
            match cmd {
                IndexerCommand::Start { full } => {
                    self.handle_start(full).await;
                }
                IndexerCommand::Stop => {
                    self.handle_stop().await;
                }
                IndexerCommand::Pause => {
                    self.handle_pause().await;
                }
                IndexerCommand::Resume => {
                    self.handle_resume().await;
                }
            }
        }
    }

    async fn handle_start(&mut self, full: bool) {
        let current_status = self.state_tx.borrow().status.clone();
        if current_status == IndexerStatus::Running || current_status == IndexerStatus::Paused {
            warn!("Indexer already running, ignoring start command");
            return;
        }

        info!("Starting indexer run (full={full})");
        self.cancel_flag.store(false, Ordering::Relaxed);
        self.pause_flag.store(false, Ordering::Relaxed);

        // Pre-record run in DB to get an ID
        let run_id = match self.app_state.db.call(move |conn| {
            crate::indexer::db_ops::record_run_start(conn, full)
        }).await {
            Ok(id) => Some(id),
            Err(e) => {
                error!("Failed to record run start: {e:#}");
                None
            }
        };

        // Reset state
        self.state_tx.send_modify(|s| {
            s.status = IndexerStatus::Running;
            s.progress = IndexProgress::default();
            s.latest_report = None;
            s.error_message = None;
            s.run_id = run_id;
        });

        let state_tx = self.state_tx.clone();
        let cancel_flag = self.cancel_flag.clone();
        let pause_flag = self.pause_flag.clone();
        let app_state = self.app_state.clone();

        // Prepare config
        let db_path = app_state.db.db_path().to_path_buf();
        let backup_dir = app_state.config.resolved_backup_dir();
        let backup_mode = app_state.config.backup_mode;
        let notify_tx = app_state.notifications.clone();
        let skip_dirs = app_state.config.indexer.skip_dirs.clone();
        let privacy = app_state.config.privacy.clone();
        let mut sources = app_state.config.resolved_sources();

        // Auto-discover extra sources
        let extras = crate::indexer::scanner::discover_extra_sources();
        for extra in extras {
            if !sources.iter().any(|(_, p, _, _)| p == &extra.1) {
                sources.push((extra.0, extra.1, extra.2, None));
            }
        }

        tokio::task::spawn_blocking(move || {
            let config = IndexConfig {
                sources,
                db_path,
                backup_dir,
                backup_mode,
                full,
                verbose: false,
                skip_dirs,
                exclude_paths: privacy.exclude_paths,
                redact_secrets: privacy.redact_secrets,
                redaction_patterns: privacy.redaction_patterns,
                retention_days: privacy.retention_days,
                progress_tx: Some(state_tx.clone()),
                cancel_flag: Some(cancel_flag.clone()),
                pause_flag: Some(pause_flag),
                notify_tx: Some(notify_tx.clone()),
                run_id,
            };

            let result = crate::indexer::run_index(config);

            let was_cancelled = cancel_flag.load(Ordering::Relaxed);
            state_tx.send_modify(|s| {
                match result {
                    Ok(report) => {
                        if was_cancelled {
                            s.status = IndexerStatus::Cancelled;
                            notifications::notify(&notify_tx, NotificationLevel::Warn, "Indexing cancelled");
                        } else {
                            s.status = IndexerStatus::Completed;
                        }
                        s.latest_report = Some(report);
                    }
                    Err(e) => {
                        s.status = IndexerStatus::Failed;
                        let msg = format!("{e:#}");
                        s.error_message = Some(msg.clone());
                        notifications::notify(&notify_tx, NotificationLevel::Error, format!("Indexing failed: {msg}"));
                    }
                }
            });
        });
    }

    async fn handle_stop(&mut self) {
        let status = self.state_tx.borrow().status.clone();
        if status == IndexerStatus::Running || status == IndexerStatus::Paused {
            info!("Stopping indexer");
            self.pause_flag.store(false, Ordering::Relaxed);
            self.cancel_flag.store(true, Ordering::Relaxed);
        }
    }

    async fn handle_pause(&mut self) {
        if self.state_tx.borrow().status == IndexerStatus::Running {
            info!("Pausing indexer");
            self.pause_flag.store(true, Ordering::Relaxed);
            self.state_tx.send_modify(|s| s.status = IndexerStatus::Paused);
        }
    }

    async fn handle_resume(&mut self) {
        if self.state_tx.borrow().status == IndexerStatus::Paused {
            info!("Resuming indexer");
            self.pause_flag.store(false, Ordering::Relaxed);
            self.state_tx.send_modify(|s| s.status = IndexerStatus::Running);
        }
    }
}
