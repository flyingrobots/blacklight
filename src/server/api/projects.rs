use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};

use crate::server::errors::AppError;
use crate::server::queries::projects;
use crate::server::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/projects", get(list_projects))
}

async fn list_projects(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state.db.call(projects::get_projects).await?;
    Ok(Json(serde_json::to_value(result)?))
}
