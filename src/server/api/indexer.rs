use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use rusqlite::params;

use crate::error::BlacklightError;
use crate::server::responses::IndexerStatusResponse;
use crate::server::state::{AppState, IndexerCommand};

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
    State(_state): State<AppState>,
) -> Result<Json<Vec<String>>, BlacklightError> {
    // TODO: Implement actor-based log buffering or use a different mechanism
    Ok(Json(vec![]))
}

async fn status(State(state): State<AppState>) -> Result<Json<IndexerStatusResponse>, BlacklightError> {
    let current = state.indexer.borrow().clone();
    
    let outdated_count = state.db.call(|conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE index_version < ?1",
            params![crate::INDEX_VERSION],
            |row| row.get(0)
        )?;
        Ok(count)
    }).await?;

    let resp = IndexerStatusResponse {
        status: current.status,
        progress: current.progress,
        latest_report: current.latest_report,
        error_message: current.error_message,
        required_version: crate::INDEX_VERSION,
        outdated_count,
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
) -> Result<(StatusCode, Json<serde_json::Value>), BlacklightError> {
    state.indexer_tx.send(IndexerCommand::Start { full: params.full })
        .await
        .map_err(|e| BlacklightError::Internal(format!("Failed to send start command: {e}")))?;

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "message": "Indexing started" })),
    ))
}

async fn stop(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, BlacklightError> {
    state.indexer_tx.send(IndexerCommand::Stop)
        .await
        .map_err(|e| BlacklightError::Internal(format!("Failed to send stop command: {e}")))?;

    Ok(Json(serde_json::json!({ "message": "Cancellation requested" })))
}

async fn pause(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, BlacklightError> {
    state.indexer_tx.send(IndexerCommand::Pause)
        .await
        .map_err(|e| BlacklightError::Internal(format!("Failed to send pause command: {e}")))?;

    Ok(Json(serde_json::json!({ "message": "Indexer paused" })))
}

async fn resume(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, BlacklightError> {
    state.indexer_tx.send(IndexerCommand::Resume)
        .await
        .map_err(|e| BlacklightError::Internal(format!("Failed to send resume command: {e}")))?;

    Ok(Json(serde_json::json!({ "message": "Indexer resumed" })))
}
