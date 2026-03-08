use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::atomic::Ordering;

use crate::error::BlacklightError;
use crate::server::state::{AppState, EnricherStatus};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/enrichment/status", get(status))
        .route("/enrichment/start", post(start))
        .route("/enrichment/stop", post(stop))
        .route("/enrichment/pending-count", get(pending_count))
        .route("/enrichment/logs", get(logs))
}

async fn logs(
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, BlacklightError> {
    let guard = state.enricher.lock().await;
    let lines = guard.log_lines.lock().unwrap().clone();
    Ok(Json(lines))
}

async fn status(State(state): State<AppState>) -> Result<Json<serde_json::Value>, BlacklightError> {
    let guard = state.enricher.lock().await;
    let progress = guard.progress.clone();

    let outdated_count = state.db.call(|conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE (enrich_version IS NULL OR enrich_version < ?1) AND source_kind = 'claude'",
            rusqlite::params![crate::ENRICH_VERSION],
            |row| row.get(0)
        )?;
        Ok(count)
    }).await?;

    Ok(Json(serde_json::json!({
        "status": guard.status,
        "sessions_total": progress.total.load(Ordering::Relaxed),
        "sessions_done": progress.done.load(Ordering::Relaxed),
        "sessions_failed": progress.failed.load(Ordering::Relaxed),
        "latest_report": guard.latest_report,
        "error_message": guard.error_message,
        "required_version": crate::ENRICH_VERSION,
        "outdated_count": outdated_count,
    })))
}

#[derive(Deserialize)]
struct StartParams {
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    concurrency: Option<usize>,
    #[serde(default)]
    force: bool,
}

async fn start(
    State(state): State<AppState>,
    Json(params): Json<StartParams>,
) -> Result<(StatusCode, Json<serde_json::Value>), BlacklightError> {
    let mut guard = state.enricher.lock().await;

    if guard.status == EnricherStatus::Running {
        return Err(BlacklightError::Parse("Enricher is already running".to_string()));
    }

    guard.reset_for_run();

    let progress = guard.progress.clone();
    let cancel_flag = guard.cancel_flag.clone();
    let enricher_state = state.enricher.clone();
    let db_path = state.db.db_path().to_path_buf();
    let notify_tx = state.notifications.clone();
    let log_lines = guard.log_lines.clone();

    let enrich_cfg = &state.config.enrichment;
    let config = crate::enrich::EnrichConfig {
        db_path,
        limit: params.limit,
        concurrency: params.concurrency.unwrap_or(enrich_cfg.concurrency),
        force: params.force,
        ollama_url: enrich_cfg.ollama_url.clone(),
        ollama_model: enrich_cfg.ollama_model.clone(),
        google_api_key: enrich_cfg.google_api_key.clone(),
        auto_approve_threshold: enrich_cfg.auto_approve_threshold,
        preferred_backend: enrich_cfg.preferred_backend.clone(),
        progress: Some(progress),
        cancel_flag: Some(cancel_flag.clone()),
        log_lines: Some(log_lines),
    };

    tokio::task::spawn(async move {
        let result = crate::enrich::run_enrich(config).await;

        let mut guard = enricher_state.lock().await;
        let was_cancelled = cancel_flag.load(Ordering::Relaxed);
        match result {
            Ok(report) => {
                if was_cancelled {
                    guard.status = EnricherStatus::Cancelled;
                } else {
                    guard.status = EnricherStatus::Completed;
                    crate::notifications::notify(
                        &notify_tx,
                        crate::notifications::NotificationLevel::Info,
                        format!(
                            "Enrichment complete: {} enriched, {} failed",
                            report.enriched, report.failed
                        ),
                    );

                    // Check for pending reviews
                    let db = crate::db::open(crate::db::default_db_path().as_path());
                    if let Ok(mut conn) = db {
                        if let Ok(pending) = crate::enrich::pending_review_count(&mut conn) {
                            if pending > 0 {
                                crate::notifications::notify(
                                    &notify_tx,
                                    crate::notifications::NotificationLevel::Info,
                                    format!("{pending} enrichment(s) need review"),
                                );
                            }
                        }
                    }
                }
                guard.latest_report = Some(report);
            }
            Err(e) => {
                guard.status = EnricherStatus::Failed;
                let msg = format!("{e:#}");
                guard.error_message = Some(msg.clone());
                crate::notifications::notify(
                    &notify_tx,
                    crate::notifications::NotificationLevel::Error,
                    format!("Enrichment failed: {msg}"),
                );
            }
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "message": "Enrichment started" })),
    ))
}

async fn stop(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, BlacklightError> {
    let guard = state.enricher.lock().await;
    guard.cancel_flag.store(true, Ordering::Relaxed);
    Ok(Json(serde_json::json!({ "message": "Cancellation requested" })))
}

async fn pending_count(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, BlacklightError> {
    let count = state
        .db
        .call(|conn| crate::enrich::pending_review_count(conn))
        .await?;

    Ok(Json(serde_json::json!({ "count": count })))
}
