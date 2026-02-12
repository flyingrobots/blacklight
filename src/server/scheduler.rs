use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::enrich::EnrichConfig;
use crate::indexer::IndexConfig;
use crate::notifications::{self, NotificationLevel};
use crate::server::state::{AppState, EnricherStatus, IndexerStatus, SchedulerHandle};

/// Spawn the background scheduler loop. Returns a handle for cancellation.
pub fn spawn_scheduler(state: AppState) -> SchedulerHandle {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let flag = cancel_flag.clone();

    tokio::spawn(async move {
        scheduler_loop(state, flag).await;
    });

    SchedulerHandle { cancel_flag }
}

async fn scheduler_loop(state: AppState, cancel_flag: Arc<AtomicBool>) {
    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            tracing::info!("scheduler cancelled");
            return;
        }

        // Read schedule config from DB
        let config = match read_schedule_config(&state).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("scheduler: failed to read config: {e:#}");
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                continue;
            }
        };

        if !config.enabled {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            continue;
        }

        // Sleep for the configured interval, checking cancel every second
        let interval_secs = (config.interval_minutes as u64) * 60;
        for _ in 0..interval_secs {
            if cancel_flag.load(Ordering::Relaxed) {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        if cancel_flag.load(Ordering::Relaxed) {
            return;
        }

        // Run indexing if not already running
        run_scheduled_index(&state).await;

        // Run enrichment if enabled and not already running
        if config.run_enrichment {
            run_scheduled_enrichment(&state, config.enrichment_concurrency).await;
        }
    }
}

struct ScheduleConfig {
    enabled: bool,
    interval_minutes: i32,
    run_enrichment: bool,
    enrichment_concurrency: i32,
}

async fn read_schedule_config(state: &AppState) -> anyhow::Result<ScheduleConfig> {
    state
        .db
        .call(|conn| {
            let mut stmt = conn.prepare(
                "SELECT enabled, interval_minutes, run_enrichment, enrichment_concurrency
                 FROM schedule_config WHERE id = 1",
            )?;
            let config = stmt.query_row([], |row| {
                Ok(ScheduleConfig {
                    enabled: row.get::<_, i32>(0)? != 0,
                    interval_minutes: row.get(1)?,
                    run_enrichment: row.get::<_, i32>(2)? != 0,
                    enrichment_concurrency: row.get(3)?,
                })
            })?;
            Ok(config)
        })
        .await
}

async fn run_scheduled_index(state: &AppState) {
    // Skip if already running
    {
        let guard = state.indexer.lock().await;
        if guard.status == IndexerStatus::Running {
            tracing::info!("scheduler: indexer already running, skipping");
            return;
        }
    }

    notifications::notify(
        &state.notifications,
        NotificationLevel::Info,
        "Scheduled indexing started",
    );

    let mut guard = state.indexer.lock().await;
    guard.reset_for_run();

    let progress = guard.progress.clone();
    let cancel_flag = guard.cancel_flag.clone();
    let pause_flag = guard.pause_flag.clone();
    let indexer_state = state.indexer.clone();
    let source_dir = state.source_dir.clone();
    let db_path = state.db.db_path().to_path_buf();
    let notify_tx = state.notifications.clone();

    drop(guard);

    let handle = tokio::task::spawn_blocking(move || {
        let extra_dirs = crate::indexer::scanner::discover_extra_sources();
        let config = IndexConfig {
            claude_dir: source_dir,
            extra_dirs,
            db_path,
            full: false,
            verbose: false,
            progress: Some(progress),
            cancel_flag: Some(cancel_flag.clone()),
            pause_flag: Some(pause_flag),
            notify_tx: Some(notify_tx.clone()),
        };

        let result = crate::indexer::run_index(config);

        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let mut guard = indexer_state.lock().await;
            let was_cancelled = cancel_flag.load(Ordering::Relaxed);
            match result {
                Ok(report) => {
                    if was_cancelled {
                        guard.status = IndexerStatus::Cancelled;
                    } else {
                        guard.status = IndexerStatus::Completed;
                        notifications::notify(
                            &notify_tx,
                            NotificationLevel::Info,
                            format!(
                                "Scheduled indexing complete: {} sessions, {} messages",
                                report.sessions_parsed, report.messages_processed
                            ),
                        );
                    }
                    guard.latest_report = Some(report);
                }
                Err(e) => {
                    guard.status = IndexerStatus::Failed;
                    let msg = format!("{e:#}");
                    guard.error_message = Some(msg.clone());
                    notifications::notify(
                        &notify_tx,
                        NotificationLevel::Error,
                        format!("Scheduled indexing failed: {msg}"),
                    );
                }
            }
        });
    });

    // Wait for indexing to finish before returning
    let _ = handle.await;
}

async fn run_scheduled_enrichment(state: &AppState, concurrency: i32) {
    // Skip if already running
    {
        let guard = state.enricher.lock().await;
        if guard.status == EnricherStatus::Running {
            tracing::info!("scheduler: enricher already running, skipping");
            return;
        }
    }

    notifications::notify(
        &state.notifications,
        NotificationLevel::Info,
        "Scheduled enrichment started",
    );

    {
        let mut guard = state.enricher.lock().await;
        guard.reset_for_run();
    }

    let enricher_state = state.enricher.clone();
    let db_path = state.db.db_path().to_path_buf();
    let notify_tx = state.notifications.clone();

    let config = EnrichConfig {
        db_path,
        limit: None,
        concurrency: concurrency as usize,
        force: false,
    };

    let result = crate::enrich::run_enrich(config).await;

    let mut guard = enricher_state.lock().await;
    match result {
        Ok(report) => {
            guard.status = EnricherStatus::Completed;
            notifications::notify(
                &notify_tx,
                NotificationLevel::Info,
                format!(
                    "Scheduled enrichment complete: {} enriched, {} failed",
                    report.enriched, report.failed
                ),
            );

            // Check for pending reviews
            if let Ok(pending) = state
                .db
                .call(crate::enrich::pending_review_count)
                .await
            {
                if pending > 0 {
                    notifications::notify(
                        &notify_tx,
                        NotificationLevel::Info,
                        format!("{pending} enrichment(s) need review"),
                    );
                }
            }

            guard.sessions_done = report.enriched;
            guard.sessions_failed = report.failed;
            guard.sessions_total = report.total_candidates;
            guard.latest_report = Some(report);
        }
        Err(e) => {
            guard.status = EnricherStatus::Failed;
            let msg = format!("{e:#}");
            guard.error_message = Some(msg.clone());
            notifications::notify(
                &notify_tx,
                NotificationLevel::Error,
                format!("Scheduled enrichment failed: {msg}"),
            );
        }
    }
}
