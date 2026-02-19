use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::server::errors::AppError;
use crate::server::state::{AppState, MigrationStatus};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/migration/status", get(status))
        .route("/migration/start", post(start))
}

async fn status(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let guard = state.migration.lock().await;
    let progress = guard.progress.lock().unwrap().clone();

    let pending_count = state.db.call(|conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE fingerprint IS NULL 
             OR id NOT IN (SELECT session_id FROM session_backups)",
            [],
            |row| row.get(0)
        )?;
        Ok(count)
    }).await?;

    Ok(Json(serde_json::json!({
        "status": guard.status,
        "progress": progress,
        "error_message": guard.error_message,
        "pending_count": pending_count,
    })))
}

async fn start(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let mut guard = state.migration.lock().await;

    if guard.status == MigrationStatus::Running {
        return Err(AppError::bad_request("Migration is already running"));
    }

    guard.status = MigrationStatus::Running;
    guard.error_message = None;
    {
        let mut p = guard.progress.lock().unwrap();
        p.total_sessions = 0;
        p.backed_up = 0;
        p.fingerprints_updated = 0;
    }

    let db = state.db.clone();
    let migration_state = state.migration.clone();
    let backup_dir = state.config.resolved_backup_dir();
    let backup_mode = state.config.backup_mode;

    tokio::task::spawn(async move {
        let result = crate::indexer::gemini::run_v4_migration(db, migration_state.clone(), backup_dir, backup_mode).await;
        
        let mut guard = migration_state.lock().await;
        match result {
            Ok(_) => guard.status = MigrationStatus::Completed,
            Err(e) => {
                guard.status = MigrationStatus::Failed;
                guard.error_message = Some(format!("{e:#}"));
            }
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "message": "Migration started" })),
    ))
}
