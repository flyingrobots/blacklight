use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};

use crate::server::errors::AppError;
use crate::server::params::{MessageListParams, SessionListParams};
use crate::server::queries::{messages, sessions};
use crate::server::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/sessions", get(list_sessions))
        .route("/sessions/{id}", get(get_session))
        .route("/sessions/{id}/messages", get(get_messages))
        .route("/sessions/{id}/tools", get(get_tools))
        .route("/sessions/{id}/files", get(get_files))
}

async fn list_sessions(
    State(state): State<AppState>,
    Query(params): Query<SessionListParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .db
        .call(move |conn| {
            sessions::list_sessions(
                conn,
                params.project.as_deref(),
                params.from.as_deref(),
                params.to.as_deref(),
                params.limit,
                params.offset,
            )
        })
        .await?;

    Ok(Json(serde_json::to_value(result)?))
}

async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .db
        .call(move |conn| sessions::get_session(conn, &id))
        .await?;

    match result {
        Some(session) => Ok(Json(serde_json::to_value(session)?)),
        None => Err(AppError::not_found("session not found")),
    }
}

async fn get_messages(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<MessageListParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .db
        .call(move |conn| messages::get_messages(conn, &id, params.limit, params.offset))
        .await?;

    Ok(Json(serde_json::to_value(result)?))
}

async fn get_tools(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .db
        .call(move |conn| sessions::get_session_tools(conn, &id))
        .await?;

    Ok(Json(serde_json::to_value(result)?))
}

async fn get_files(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .db
        .call(move |conn| sessions::get_session_files(conn, &id))
        .await?;

    Ok(Json(serde_json::to_value(result)?))
}
