use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::error::BlacklightError;
use crate::server::params::LimitParams;
use crate::server::queries;
use crate::server::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/review", get(list_review))
        .route("/review/{id}/approve", post(approve))
        .route("/review/{id}/reject", post(reject))
        .route("/review/approve-all", post(approve_all))
}

async fn list_review(
    State(state): State<AppState>,
    Query(params): Query<LimitParams>,
) -> Result<Json<serde_json::Value>, BlacklightError> {
    let result = state
        .db
        .call(move |conn| {
            queries::review::list_pending(conn, params.limit, params.offset)
        })
        .await?;

    Ok(Json(serde_json::to_value(result)?))
}

async fn approve(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, BlacklightError> {
    state
        .db
        .write(move |conn| {
            queries::review::approve_session(conn, &id)
        })
        .await?;

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

async fn reject(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, BlacklightError> {
    state
        .db
        .write(move |conn| {
            queries::review::reject_session(conn, &id)
        })
        .await?;

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

async fn approve_all(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, BlacklightError> {
    let count = state
        .db
        .write(|conn| queries::review::approve_all(conn))
        .await?;

    Ok(Json(serde_json::json!({ "message": "Approved all", "count": count })))
}
