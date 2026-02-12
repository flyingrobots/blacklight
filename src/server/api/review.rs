use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::server::errors::AppError;
use crate::server::queries;
use crate::server::responses::{Paginated, ReviewItem};
use crate::server::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/review", get(list_pending))
        .route("/review/{session_id}/approve", post(approve))
        .route("/review/{session_id}/reject", post(reject))
        .route("/review/approve-all", post(approve_all))
}

#[derive(Deserialize)]
struct ListParams {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    50
}

async fn list_pending(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<ListParams>,
) -> Result<Json<Paginated<ReviewItem>>, AppError> {
    let result = state
        .db
        .call(move |conn| queries::review::list_pending(conn, params.limit, params.offset))
        .await?;

    Ok(Json(result))
}

async fn approve(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let sid = session_id.clone();
    let updated = state
        .db
        .call(move |conn| queries::review::approve_session(conn, &sid))
        .await?;

    if !updated {
        return Err(AppError::not_found("No pending enrichment found for this session"));
    }

    Ok(Json(serde_json::json!({ "message": "Approved", "session_id": session_id })))
}

async fn reject(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let sid = session_id.clone();
    let updated = state
        .db
        .call(move |conn| queries::review::reject_session(conn, &sid))
        .await?;

    if !updated {
        return Err(AppError::not_found("No pending enrichment found for this session"));
    }

    Ok(Json(serde_json::json!({ "message": "Rejected", "session_id": session_id })))
}

async fn approve_all(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let count = state
        .db
        .call(queries::review::approve_all)
        .await?;

    Ok(Json(serde_json::json!({ "message": "Approved all", "count": count })))
}
