use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::IntoResponse;
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
        .route("/sessions/{id}/raw", get(get_raw))
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

async fn get_raw(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Must find a backup in CAS
    let backup_info = state
        .db
        .call(move |conn| {
            let row: Option<String> = conn
                .query_row(
                    "SELECT content_hash FROM session_backups WHERE session_id = ?1",
                    rusqlite::params![id],
                    |row| row.get(0),
                )
                .ok();
            Ok(row)
        })
        .await?;

    let hash = backup_info.ok_or_else(|| AppError::not_found("session not backed up in CAS"))?;
    let backup_dir = state.config.resolved_backup_dir();

    match state.config.backup_mode {
        crate::config::BackupMode::Simple => {
            let path = backup_dir.join(&hash);
            let content = tokio::fs::read_to_string(path)
                .await
                .map_err(|e| AppError::internal(format!("failed to read CAS backup: {e}")))?;
            Ok((
                [(header::CONTENT_TYPE, "application/x-ndjson; charset=utf-8")],
                content,
            ))
        }
        crate::config::BackupMode::GitCas => {
            // Snappy materialized cache check
            let materialized_dir = state
                .config
                .resolved_db_path()
                .parent()
                .ok_or_else(|| AppError::internal("no db parent"))?
                .join("materialized");
            if !materialized_dir.exists() {
                std::fs::create_dir_all(&materialized_dir)
                    .map_err(|e| AppError::internal(format!("failed to create cache: {e}")))?;
            }

            let cache_path = materialized_dir.join(&hash);
            if cache_path.exists() {
                let content = tokio::fs::read_to_string(cache_path).await.map_err(|e| {
                    AppError::internal(format!("failed to read materialized cache: {e}"))
                })?;
                return Ok((
                    [(header::CONTENT_TYPE, "application/x-ndjson; charset=utf-8")],
                    content,
                ));
            }

            // Not in cache, restore from git-cas
            let output = tokio::process::Command::new("git")
                .args([
                    "cas",
                    "restore",
                    "--oid",
                    &hash,
                    "--out",
                    cache_path.to_string_lossy().as_ref(),
                ])
                .current_dir(&backup_dir)
                .output()
                .await
                .map_err(|e| AppError::internal(format!("failed to run git cas restore: {e}")))?;

            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                return Err(AppError::internal(format!("git cas restore failed: {err}")));
            }

            let content = tokio::fs::read_to_string(cache_path)
                .await
                .map_err(|e| AppError::internal(format!("failed to read restored file: {e}")))?;

            Ok((
                [(header::CONTENT_TYPE, "application/x-ndjson; charset=utf-8")],
                content,
            ))
        }
    }
}
