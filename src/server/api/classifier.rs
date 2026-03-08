use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::error::BlacklightError;
use crate::server::state::{AppState, ClassifierCommand, ClassifierState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/classifier/status", get(status))
        .route("/classifier/start", post(start))
        .route("/classifier/stop", post(stop))
}

async fn status(State(state): State<AppState>) -> Result<Json<ClassifierState>, BlacklightError> {
    let current = state.classifier.borrow().clone();
    Ok(Json(current))
}

#[derive(Deserialize)]
struct StartClassifierParams {
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    force: bool,
}

async fn start(
    State(state): State<AppState>,
    Json(params): Json<StartClassifierParams>,
) -> Result<(StatusCode, Json<serde_json::Value>), BlacklightError> {
    state.classifier_tx.send(ClassifierCommand::Start { 
        limit: params.limit, 
        force: params.force 
    })
    .await
    .map_err(|e| BlacklightError::Internal(format!("Failed to send start command: {e}")))?;

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "message": "Classification started" })),
    ))
}

async fn stop(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, BlacklightError> {
    state.classifier_tx.send(ClassifierCommand::Stop)
    .await
    .map_err(|e| BlacklightError::Internal(format!("Failed to send stop command: {e}")))?;

    Ok(Json(serde_json::json!({ "message": "Stop requested" })))
}
