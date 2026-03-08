use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::error::BlacklightError;
use crate::server::params::LimitParams;
use crate::server::state::AppState;
use crate::digest::{self, DigestConfig};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/digests", get(list_digests))
        .route("/digests/generate", post(generate_digest))
}

async fn list_digests(
    State(state): State<AppState>,
    Query(params): Query<LimitParams>,
) -> Result<Json<serde_json::Value>, BlacklightError> {
    let result = state
        .db
        .call(move |conn| {
            digest::list_digests(conn, params.limit, params.offset)
        })
        .await?;

    Ok(Json(serde_json::to_value(result)?))
}

#[derive(Deserialize)]
pub struct GenerateDigestParams {
    pub start_date: String,
    pub end_date: String,
}

async fn generate_digest(
    State(state): State<AppState>,
    Json(params): Json<GenerateDigestParams>,
) -> Result<Json<serde_json::Value>, BlacklightError> {
    let db_path = state.db.db_path().to_path_buf();
    let config = state.config.enrichment.clone();
    
    let digest_config = DigestConfig {
        db_path,
        ollama_url: config.ollama_url,
        ollama_model: config.ollama_model,
        google_api_key: config.google_api_key,
        preferred_backend: config.preferred_backend,
    };

    let result = digest::generate_weekly_digest(digest_config, params.start_date, params.end_date).await?;

    Ok(Json(serde_json::to_value(result)?))
}
