use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::atomic::Ordering;

use axum::extract::Path;

use crate::enrich::EnrichConfig;
use crate::notifications::{self, NotificationLevel};
use crate::server::errors::AppError;
use crate::server::responses::EnricherStatusResponse;
use crate::server::state::{AppState, EnricherStatus};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/enrichment/status", get(status))
        .route("/enrichment/start", post(start))
        .route("/enrichment/stop", post(stop))
        .route("/enrichment/pending-count", get(pending_count))
        .route("/enrichment/logs", get(logs))
        .route("/enrichment/session/{session_id}", post(enrich_single))
}

async fn status(
    State(state): State<AppState>,
) -> Result<Json<EnricherStatusResponse>, AppError> {
    let guard = state.enricher.lock().await;
    let resp = EnricherStatusResponse {
        status: guard.status.clone(),
        sessions_total: guard.progress.total.load(Ordering::Relaxed),
        sessions_done: guard.progress.done.load(Ordering::Relaxed),
        sessions_failed: guard.progress.failed.load(Ordering::Relaxed),
        latest_report: guard.latest_report.clone(),
        error_message: guard.error_message.clone(),
    };
    Ok(Json(resp))
}

#[derive(Deserialize)]
struct StartParams {
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default = "default_concurrency")]
    concurrency: usize,
    #[serde(default)]
    force: bool,
}

fn default_concurrency() -> usize {
    5
}

async fn start(
    State(state): State<AppState>,
    Json(params): Json<StartParams>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let mut guard = state.enricher.lock().await;

    if guard.status == EnricherStatus::Running {
        return Err(AppError::bad_request("Enricher is already running"));
    }

    guard.reset_for_run();
    let progress = guard.progress.clone();
    let cancel_flag = guard.cancel_flag.clone();
    let log_lines = guard.log_lines.clone();

    let enricher_state = state.enricher.clone();
    let db_path = state.db.db_path().to_path_buf();
    let notify_tx = state.notifications.clone();
    let db = state.db.clone();

    drop(guard);

    let enrich_cfg = &state.config.enrichment;
    let config = EnrichConfig {
        db_path,
        limit: params.limit,
        concurrency: params.concurrency,
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

    let cancel_flag_check = cancel_flag;

    tokio::spawn(async move {
        let result = crate::enrich::run_enrich(config).await;

        let mut guard = enricher_state.lock().await;
        let was_cancelled = cancel_flag_check.load(Ordering::Relaxed);
        match result {
            Ok(report) => {
                if was_cancelled {
                    guard.status = EnricherStatus::Cancelled;
                    notifications::notify(
                        &notify_tx,
                        NotificationLevel::Warn,
                        "Enrichment cancelled",
                    );
                    guard.latest_report = Some(report);
                    return;
                }
                guard.status = EnricherStatus::Completed;
                notifications::notify(
                    &notify_tx,
                    NotificationLevel::Info,
                    format!(
                        "Enrichment complete: {} enriched, {} failed",
                        report.enriched, report.failed
                    ),
                );

                // Notify about pending reviews
                if let Ok(pending) = db
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

                guard.latest_report = Some(report);
            }
            Err(e) => {
                guard.status = EnricherStatus::Failed;
                let msg = format!("{e:#}");
                guard.error_message = Some(msg.clone());
                notifications::notify(
                    &notify_tx,
                    NotificationLevel::Error,
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
) -> Result<Json<serde_json::Value>, AppError> {
    let guard = state.enricher.lock().await;

    if guard.status != EnricherStatus::Running {
        return Err(AppError::bad_request("Enricher is not running"));
    }

    guard.cancel_flag.store(true, Ordering::Relaxed);

    Ok(Json(serde_json::json!({ "message": "Cancellation requested" })))
}

async fn pending_count(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let count = state
        .db
        .call(crate::enrich::pending_review_count)
        .await?;

    Ok(Json(serde_json::json!({ "count": count })))
}

async fn logs(
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, AppError> {
    let guard = state.enricher.lock().await;
    let lines = guard.log_lines.lock().unwrap().clone();
    Ok(Json(lines))
}

/// Enrich a single session by ID (runs in background).
async fn enrich_single(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let db_path = state.db.db_path().to_path_buf();
    let ollama_url = state.config.enrichment.ollama_url.clone();
    let auto_approve_threshold = state.config.enrichment.auto_approve_threshold;

    let sid = session_id.clone();
    tokio::spawn(async move {
        // Call the lower-level pieces directly for a single session.
        let conn = match crate::db::open(&db_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("enrich_single: failed to open DB: {e:#}");
                return;
            }
        };

        // Build digest for this specific session
        let digest = match crate::enrich::build_session_digest_pub(&conn, &sid) {
            Ok(Some(d)) => d,
            Ok(None) => {
                tracing::warn!("enrich_single: session {sid} has no data");
                return;
            }
            Err(e) => {
                tracing::error!("enrich_single: failed to build digest: {e:#}");
                return;
            }
        };

        match crate::enrich::call_model_pub(&digest, &ollama_url).await {
            Ok((result, model_name)) => {
                if let Err(e) = crate::enrich::store_enrichment_pub(&conn, &sid, &result, &model_name, auto_approve_threshold) {
                    tracing::error!("enrich_single: failed to store: {e:#}");
                } else {
                    tracing::info!("enrich_single: enriched session {sid} using {model_name}: \"{}\"", result.title);
                }
            }
            Err(e) => {
                tracing::error!("enrich_single: model call failed for {sid}: {e:#}");
            }
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "message": "Enrichment started", "session_id": session_id })),
    ))
}
