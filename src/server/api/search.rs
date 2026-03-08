use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use crate::error::BlacklightError;
use crate::server::params::SearchParams;
use crate::server::queries::search;
use crate::server::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/search", get(search_content))
}

async fn search_content(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<serde_json::Value>, BlacklightError> {
    if params.q.is_empty() {
        return Err(BlacklightError::Parse("query parameter 'q' is required".to_string()));
    }

    let result = state
        .db
        .call(move |conn| {
            search::search_content(
                conn,
                &params.q,
                params.kind.as_deref(),
                params.project.as_deref(),
                params.limit,
                params.offset,
            )
        })
        .await?;

    Ok(Json(serde_json::to_value(result)?))
}
