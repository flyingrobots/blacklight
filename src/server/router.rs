use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};

use super::api;
use super::embedded::static_handler;
use super::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_routes = Router::new()
        .merge(api::sessions::routes())
        .merge(api::search::routes())
        .merge(api::analytics::routes())
        .merge(api::projects::routes())
        .merge(api::content::routes())
        .merge(api::files::routes())
        .merge(api::storage::routes());

    Router::new()
        .nest("/api", api_routes)
        .fallback(static_handler)
        .layer(CompressionLayer::new())
        .layer(cors)
        .with_state(state)
}
