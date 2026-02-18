pub mod api;
mod embedded;
pub mod errors;
pub mod params;
pub mod queries;
pub mod responses;
pub mod router;
pub mod scheduler;
pub mod state;

use anyhow::Result;
use std::path::Path;
use std::sync::Arc;

use crate::config::BlacklightConfig;
use crate::notifications;
use state::{AppState, DbPool, EnricherState, IndexerState, MigrationState};

/// Start the web server on the given port.
pub async fn start_server(
    db_path: &Path,
    port: u16,
    no_open: bool,
    config: &BlacklightConfig,
) -> Result<()> {
    let pool = DbPool::new(db_path, 4)?;
    let state = AppState {
        db: Arc::new(pool),
        config: Arc::new(config.clone()),
        indexer: Arc::new(tokio::sync::Mutex::new(IndexerState::default())),
        enricher: Arc::new(tokio::sync::Mutex::new(EnricherState::default())),
        migration: Arc::new(tokio::sync::Mutex::new(MigrationState::default())),
        scheduler: Arc::new(tokio::sync::Mutex::new(None)),
        notifications: notifications::create_channel(),
    };

    // Spawn the background scheduler
    let handle = scheduler::spawn_scheduler(state.clone());
    *state.scheduler.lock().await = Some(handle);

    let app = router::build_router(state);

    let addr = format!("0.0.0.0:{port}");
    let url = format!("http://{addr}");

    tracing::info!("starting server at {url}");

    if !no_open {
        if let Err(e) = open::that(&url) {
            tracing::warn!("failed to open browser: {e}");
        }
    }

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
