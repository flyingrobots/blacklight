pub mod api;
mod classifier_actor;
mod embedded;
mod indexer_actor;
pub mod params;
pub mod queries;
pub mod responses;
pub mod router;
pub mod scheduler;
pub mod state;

use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};

use crate::config::BlacklightConfig;
use crate::notifications;
use state::{AppState, DbPool, EnricherState, IndexerState, MigrationState, ClassifierState};
use indexer_actor::IndexerActor;
use classifier_actor::ClassifierActor;

/// Start the web server on the given port.
pub async fn start_server(
    db_path: &Path,
    port: u16,
    no_open: bool,
    config: &BlacklightConfig,
) -> Result<()> {
    let pool = DbPool::new(db_path, 4)?;
    
    let (indexer_tx, indexer_rx) = mpsc::channel(32);
    let (state_tx, state_rx) = watch::channel(IndexerState::default());

    let (classifier_tx, classifier_rx) = mpsc::channel(32);
    let (c_state_tx, c_state_rx) = watch::channel(ClassifierState::default());

    let state = AppState {
        db: Arc::new(pool),
        config: Arc::new(config.clone()),
        indexer: state_rx,
        indexer_tx,
        classifier: c_state_rx,
        classifier_tx,
        enricher: Arc::new(tokio::sync::Mutex::new(EnricherState::default())),
        migration: Arc::new(tokio::sync::Mutex::new(MigrationState::default())),
        scheduler: Arc::new(tokio::sync::Mutex::new(None)),
        notifications: notifications::create_channel(),
    };

    // Spawn the Indexer Actor
    IndexerActor::spawn(state.clone(), indexer_rx, state_tx);

    // Spawn the Classifier Actor
    ClassifierActor::spawn(state.clone(), classifier_rx, c_state_tx);

    // Spawn the background scheduler
    let handle = scheduler::spawn_scheduler(state.clone());
    *state.scheduler.lock().await = Some(handle);

    let app = router::build_router(state);

    let addr = format!("127.0.0.1:{port}");
    let url = format!("http://{addr}");

    tracing::info!("starting server at {url}");

    if !no_open {
        if let Err(e) = open::that(&url) {
            tracing::warn!("failed to open browser: {e}");
        }
    }

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Blacklight backend is READY at {}", url);
    axum::serve(listener, app).await?;

    Ok(())
}
