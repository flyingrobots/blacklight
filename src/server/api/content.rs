use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

use crate::server::errors::AppError;
use crate::server::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/content/{hash}", get(get_content))
}

async fn get_content(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .db
        .call(move |conn| crate::content::get_blob(conn, &hash))
        .await?;

    match result {
        Some(blob) => Ok(Json(serde_json::json!({
            "hash": blob.hash,
            "content": blob.content,
            "size": blob.size,
            "kind": blob.kind,
        }))),
        None => Err(AppError::not_found("blob not found")),
    }
}
