use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

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
}

async fn status(
    State(state): State<AppState>,
) -> Result<Json<EnricherStatusResponse>, AppError> {
    let guard = state.enricher.lock().await;
    let resp = EnricherStatusResponse {
        status: guard.status.clone(),
        sessions_total: guard.sessions_total,
        sessions_done: guard.sessions_done,
        sessions_failed: guard.sessions_failed,
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

    let enricher_state = state.enricher.clone();
    let db_path = state.db.db_path().to_path_buf();
    let notify_tx = state.notifications.clone();
    let db = state.db.clone();

    drop(guard);

    let config = EnrichConfig {
        db_path,
        limit: params.limit,
        concurrency: params.concurrency,
        force: params.force,
    };

    tokio::spawn(async move {
        let result = crate::enrich::run_enrich(config).await;

        let mut guard = enricher_state.lock().await;
        match result {
            Ok(report) => {
                guard.status = EnricherStatus::Completed;
                guard.sessions_done = report.enriched;
                guard.sessions_failed = report.failed;
                guard.sessions_total = report.total_candidates;
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

    guard.cancel_flag.store(true, std::sync::atomic::Ordering::Relaxed);

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
