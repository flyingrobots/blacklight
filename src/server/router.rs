use axum::Router;
use tower_http::compression::CompressionLayer;

use super::api;
use super::embedded::static_handler;
use super::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .merge(api::sessions::routes())
        .merge(api::search::routes())
        .merge(api::analytics::routes())
        .merge(api::projects::routes())
        .merge(api::content::routes())
        .merge(api::files::routes())
        .merge(api::storage::routes())
        .merge(api::indexer::routes())
        .merge(api::enrichment::routes())
        .merge(api::review::routes())
        .merge(api::schedule::routes())
        .merge(api::migration::routes())
        .merge(api::ws::routes());

    Router::new()
        .nest("/api", api_routes)
        .fallback(static_handler)
        .layer(CompressionLayer::new())
        .with_state(state)
}
