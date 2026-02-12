use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::server::errors::AppError;
use crate::server::responses::ScheduleConfigResponse;
use crate::server::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/schedule", get(get_config))
        .route("/schedule", post(update_config))
}

async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<ScheduleConfigResponse>, AppError> {
    let config = state
        .db
        .call(|conn| {
            let mut stmt = conn.prepare(
                "SELECT enabled, interval_minutes, run_enrichment, enrichment_concurrency, updated_at
                 FROM schedule_config WHERE id = 1",
            )?;
            let config = stmt.query_row([], |row| {
                Ok(ScheduleConfigResponse {
                    enabled: row.get::<_, i32>(0)? != 0,
                    interval_minutes: row.get(1)?,
                    run_enrichment: row.get::<_, i32>(2)? != 0,
                    enrichment_concurrency: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?;
            Ok(config)
        })
        .await?;

    Ok(Json(config))
}

#[derive(Deserialize)]
struct UpdateParams {
    enabled: Option<bool>,
    interval_minutes: Option<i32>,
    run_enrichment: Option<bool>,
    enrichment_concurrency: Option<i32>,
}

async fn update_config(
    State(state): State<AppState>,
    Json(params): Json<UpdateParams>,
) -> Result<Json<ScheduleConfigResponse>, AppError> {
    let config = state
        .db
        .call(move |conn| {
            if let Some(enabled) = params.enabled {
                conn.execute(
                    "UPDATE schedule_config SET enabled = ?1, updated_at = datetime('now') WHERE id = 1",
                    rusqlite::params![enabled as i32],
                )?;
            }
            if let Some(interval) = params.interval_minutes {
                if interval < 1 {
                    anyhow::bail!("interval_minutes must be >= 1");
                }
                conn.execute(
                    "UPDATE schedule_config SET interval_minutes = ?1, updated_at = datetime('now') WHERE id = 1",
                    rusqlite::params![interval],
                )?;
            }
            if let Some(run_enrichment) = params.run_enrichment {
                conn.execute(
                    "UPDATE schedule_config SET run_enrichment = ?1, updated_at = datetime('now') WHERE id = 1",
                    rusqlite::params![run_enrichment as i32],
                )?;
            }
            if let Some(concurrency) = params.enrichment_concurrency {
                if concurrency < 1 {
                    anyhow::bail!("enrichment_concurrency must be >= 1");
                }
                conn.execute(
                    "UPDATE schedule_config SET enrichment_concurrency = ?1, updated_at = datetime('now') WHERE id = 1",
                    rusqlite::params![concurrency],
                )?;
            }

            // Return the updated config
            let mut stmt = conn.prepare(
                "SELECT enabled, interval_minutes, run_enrichment, enrichment_concurrency, updated_at
                 FROM schedule_config WHERE id = 1",
            )?;
            let config = stmt.query_row([], |row| {
                Ok(ScheduleConfigResponse {
                    enabled: row.get::<_, i32>(0)? != 0,
                    interval_minutes: row.get(1)?,
                    run_enrichment: row.get::<_, i32>(2)? != 0,
                    enrichment_concurrency: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?;
            Ok(config)
        })
        .await?;

    Ok(Json(config))
}
