use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::atomic::Ordering;

use crate::indexer::IndexConfig;
use crate::notifications::{self, NotificationLevel};
use crate::server::errors::AppError;
use crate::server::responses::IndexerStatusResponse;
use crate::server::state::{AppState, IndexerStatus};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/indexer/status", get(status))
        .route("/indexer/start", post(start))
        .route("/indexer/stop", post(stop))
        .route("/indexer/pause", post(pause))
        .route("/indexer/resume", post(resume))
        .route("/indexer/logs", get(logs))
}

async fn logs(
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, AppError> {
    let guard = state.indexer.lock().await;
    let lines = guard.log_lines.lock().unwrap().clone();
    Ok(Json(lines))
}

async fn status(State(state): State<AppState>) -> Result<Json<IndexerStatusResponse>, AppError> {
    let guard = state.indexer.lock().await;
    let progress = guard.progress.lock().unwrap().clone();
    let resp = IndexerStatusResponse {
        status: guard.status.clone(),
        progress,
        latest_report: guard.latest_report.clone(),
        error_message: guard.error_message.clone(),
    };
    Ok(Json(resp))
}

#[derive(Deserialize)]
struct StartParams {
    #[serde(default)]
    full: bool,
}

async fn start(
    State(state): State<AppState>,
    Json(params): Json<StartParams>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let mut guard = state.indexer.lock().await;

    if guard.status == IndexerStatus::Running {
        return Err(AppError::bad_request("Indexer is already running"));
    }

    guard.reset_for_run();

    let progress = guard.progress.clone();
    let cancel_flag = guard.cancel_flag.clone();
    let pause_flag = guard.pause_flag.clone();
    let indexer_state = state.indexer.clone();
    let source_dir = state.source_dir.clone();
    let db_path = state.db.db_path().to_path_buf();
    let full = params.full;
    let notify_tx = state.notifications.clone();

    drop(guard); // Release lock before spawning

    tokio::task::spawn_blocking(move || {
        let extra_dirs = crate::indexer::scanner::discover_extra_sources();
        let config = IndexConfig {
            claude_dir: source_dir,
            extra_dirs,
            db_path,
            full,
            verbose: false,
            progress: Some(progress),
            cancel_flag: Some(cancel_flag.clone()),
            pause_flag: Some(pause_flag),
            notify_tx: Some(notify_tx.clone()),
        };

        let result = crate::indexer::run_index(config);

        // Update state with result — block on the async mutex from sync context
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let mut guard = indexer_state.lock().await;
            let was_cancelled = cancel_flag.load(Ordering::Relaxed);
            match result {
                Ok(report) => {
                    if was_cancelled {
                        guard.status = IndexerStatus::Cancelled;
                        notifications::notify(&notify_tx, NotificationLevel::Warn, "Indexing cancelled");
                    } else {
                        guard.status = IndexerStatus::Completed;
                    }
                    guard.latest_report = Some(report);
                }
                Err(e) => {
                    guard.status = IndexerStatus::Failed;
                    let msg = format!("{e:#}");
                    guard.error_message = Some(msg.clone());
                    notifications::notify(&notify_tx, NotificationLevel::Error, format!("Indexing failed: {msg}"));
                }
            }
        });
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "message": "Indexing started" })),
    ))
}

async fn stop(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let guard = state.indexer.lock().await;

    if guard.status != IndexerStatus::Running && guard.status != IndexerStatus::Paused {
        return Err(AppError::bad_request("Indexer is not running"));
    }

    // Clear pause so the loop can exit, then set cancel
    guard.pause_flag.store(false, Ordering::Relaxed);
    guard.cancel_flag.store(true, Ordering::Relaxed);

    Ok(Json(serde_json::json!({ "message": "Cancellation requested" })))
}

async fn pause(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut guard = state.indexer.lock().await;

    if guard.status != IndexerStatus::Running {
        return Err(AppError::bad_request("Indexer is not running"));
    }

    guard.pause_flag.store(true, Ordering::Relaxed);
    guard.status = IndexerStatus::Paused;

    Ok(Json(serde_json::json!({ "message": "Indexer paused" })))
}

async fn resume(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut guard = state.indexer.lock().await;

    if guard.status != IndexerStatus::Paused {
        return Err(AppError::bad_request("Indexer is not paused"));
    }

    guard.pause_flag.store(false, Ordering::Relaxed);
    guard.status = IndexerStatus::Running;

    Ok(Json(serde_json::json!({ "message": "Indexer resumed" })))
}
