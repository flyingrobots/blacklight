use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, watch};
use tracing::{info, warn};

use crate::classifier::ClassifierConfig;
use crate::enrich::{EnrichProgress, push_log};
use crate::notifications::{self, NotificationLevel};
use crate::server::state::{AppState, ClassifierCommand, ClassifierState, EnricherStatus};

pub struct ClassifierActor {
    state_tx: watch::Sender<ClassifierState>,
    command_rx: mpsc::Receiver<ClassifierCommand>,
    app_state: AppState,
    cancel_flag: Arc<AtomicBool>,
}

impl ClassifierActor {
    pub fn spawn(
        app_state: AppState,
        command_rx: mpsc::Receiver<ClassifierCommand>,
        state_tx: watch::Sender<ClassifierState>,
    ) {
        let actor = Self {
            state_tx,
            command_rx,
            app_state,
            cancel_flag: Arc::new(AtomicBool::new(false)),
        };

        tokio::spawn(async move {
            actor.run().await;
        });
    }

    async fn run(mut self) {
        info!("Classifier actor started");

        while let Some(cmd) = self.command_rx.recv().await {
            match cmd {
                ClassifierCommand::Start { limit, force } => {
                    self.handle_start(limit, force).await;
                }
                ClassifierCommand::Stop => {
                    self.handle_stop().await;
                }
            }
        }
    }

    async fn handle_start(&mut self, limit: Option<usize>, force: bool) {
        let current_status = self.state_tx.borrow().status.clone();
        if current_status == EnricherStatus::Running {
            warn!("Classifier already running, ignoring start command");
            return;
        }

        info!("Starting outcome classification run (force={force})");
        self.cancel_flag.store(false, Ordering::Relaxed);

        // Reset state
        self.state_tx.send_modify(|s| {
            s.status = EnricherStatus::Running;
            s.sessions_total = 0;
            s.sessions_done = 0;
            s.sessions_failed = 0;
            s.latest_report = None;
            s.error_message = None;
        });

        let state_tx = self.state_tx.clone();
        let cancel_flag = self.cancel_flag.clone();
        let app_state = self.app_state.clone();
        let progress = Arc::new(EnrichProgress::default());
        let progress_clone = progress.clone();
        let log_lines = Arc::new(Mutex::new(Vec::new()));

        // Prepare config
        let db_path = app_state.db.db_path().to_path_buf();
        let config = &app_state.config.enrichment; // Reuse enrichment config for LLM settings
        
        let classifier_config = ClassifierConfig {
            db_path,
            limit,
            concurrency: config.concurrency,
            force,
            ollama_url: config.ollama_url.clone(),
            ollama_model: config.ollama_model.clone(),
            google_api_key: config.google_api_key.clone(),
            preferred_backend: config.preferred_backend.clone(),
            progress: Some(progress),
            cancel_flag: Some(cancel_flag),
            log_lines: Some(log_lines),
        };

        // Update total from progress once known
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
            loop {
                interval.tick().await;
                let done = progress_clone.done.load(Ordering::Relaxed);
                let failed = progress_clone.failed.load(Ordering::Relaxed);
                let total = progress_clone.total.load(Ordering::Relaxed);
                
                state_tx.send_modify(|s| {
                    s.sessions_done = done;
                    s.sessions_failed = failed;
                    s.sessions_total = total;
                });

                if state_tx.borrow().status != EnricherStatus::Running {
                    break;
                }
            }
        });

        let state_tx = self.state_tx.clone();
        let notify_tx = app_state.notifications.clone();

        tokio::spawn(async move {
            let result = crate::classifier::run_classify(classifier_config).await;

            state_tx.send_modify(|s| {
                match result {
                    Ok(report) => {
                        s.status = EnricherStatus::Completed;
                        s.latest_report = Some(report.clone());
                        notifications::notify(
                            &notify_tx, 
                            NotificationLevel::Info, 
                            format!("Outcome classification complete: {} classified", report.classified)
                        );
                    }
                    Err(e) => {
                        s.status = EnricherStatus::Failed;
                        let msg = format!("{e:#}");
                        s.error_message = Some(msg.clone());
                        notifications::notify(
                            &notify_tx, 
                            NotificationLevel::Error, 
                            format!("Outcome classification failed: {msg}")
                        );
                    }
                }
            });
        });
    }

    async fn handle_stop(&mut self) {
        if self.state_tx.borrow().status == EnricherStatus::Running {
            info!("Stopping classifier");
            self.cancel_flag.store(true, Ordering::Relaxed);
            self.state_tx.send_modify(|s| s.status = EnricherStatus::Cancelled);
        }
    }
}
