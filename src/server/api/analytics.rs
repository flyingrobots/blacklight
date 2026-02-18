use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};

use crate::server::errors::AppError;
use crate::server::params::{DateRangeParams, LimitParams};
use crate::server::queries::analytics;
use crate::server::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/analytics/overview", get(overview))
        .route("/analytics/coverage", get(coverage))
        .route("/analytics/daily", get(daily))
        .route("/analytics/models", get(models))
        .route("/analytics/tools", get(tools))
        .route("/analytics/projects", get(projects))
        .route("/analytics/outcomes", get(outcomes))
}

async fn overview(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db_path = state.db.db_path().to_string_lossy().to_string();
    let result = state
        .db
        .call(move |conn| analytics::get_overview(conn, &db_path))
        .await?;

    Ok(Json(serde_json::to_value(result)?))
}

async fn coverage(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let source_dirs: Vec<_> = state.config.resolved_sources().into_iter().map(|(_, p, _, _)| p).collect();
    let result = state
        .db
        .call(move |conn| analytics::get_coverage(conn, &source_dirs))
        .await?;

    Ok(Json(serde_json::to_value(result)?))
}

async fn daily(
    State(state): State<AppState>,
    Query(params): Query<DateRangeParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .db
        .call(move |conn| {
            analytics::get_daily_stats(conn, params.from.as_deref(), params.to.as_deref())
        })
        .await?;

    Ok(Json(serde_json::to_value(result)?))
}

async fn models(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .db
        .call(analytics::get_model_usage)
        .await?;

    Ok(Json(serde_json::to_value(result)?))
}

async fn tools(
    State(state): State<AppState>,
    Query(params): Query<LimitParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .db
        .call(move |conn| analytics::get_tool_frequency(conn, params.limit))
        .await?;

    Ok(Json(serde_json::to_value(result)?))
}

async fn projects(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .db
        .call(analytics::get_project_breakdown)
        .await?;

    Ok(Json(serde_json::to_value(result)?))
}

async fn outcomes(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .db
        .call(analytics::get_outcome_distribution)
        .await?;

    Ok(Json(serde_json::to_value(result)?))
}
