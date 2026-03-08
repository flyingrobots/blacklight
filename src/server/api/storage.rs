use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};

use crate::error::BlacklightError;
use crate::server::queries::storage;
use crate::server::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/storage", get(get_storage))
}

async fn get_storage(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, BlacklightError> {
    let result = state
        .db
        .call(storage::get_storage_overview)
        .await?;

    Ok(Json(serde_json::to_value(result)?))
}
