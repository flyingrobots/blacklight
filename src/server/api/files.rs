use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};

use crate::server::errors::AppError;
use crate::server::params::FileQueryParams;
use crate::server::queries::files;
use crate::server::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/files", get(get_files))
}

async fn get_files(
    State(state): State<AppState>,
    Query(params): Query<FileQueryParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .db
        .call(move |conn| {
            files::get_file_references(
                conn,
                params.path.as_deref(),
                params.session.as_deref(),
                params.limit,
                params.offset,
            )
        })
        .await?;

    Ok(Json(serde_json::to_value(result)?))
}
