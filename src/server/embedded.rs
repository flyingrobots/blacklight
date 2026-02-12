use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "frontend/dist/"]
struct FrontendAssets;

/// Serve static files from the embedded frontend, with SPA fallback to index.html.
pub async fn static_handler(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    // Never serve HTML for API paths — return 404 JSON instead
    if path.starts_with("api/") {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"error":"not found"}"#,
        )
            .into_response();
    }

    // Try exact file match first
    if let Some(file) = FrontendAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, mime.as_ref())],
            file.data.to_vec(),
        )
            .into_response();
    }

    // SPA fallback: serve index.html for non-API routes
    match FrontendAssets::get("index.html") {
        Some(file) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/html")],
            file.data.to_vec(),
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "frontend not built").into_response(),
    }
}
