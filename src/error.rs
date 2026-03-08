use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlacklightError {
    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Anyhow error: {0:#}")]
    Anyhow(#[from] anyhow::Error),
}

impl IntoResponse for BlacklightError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Self::Database(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {err}")),
            Self::Io(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("I/O error: {err}")),
            Self::Serialization(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Serialization error: {err}")),
            Self::Parse(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Self::Anyhow(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{err:#}")),
        };

        let body = json!({
            "error": message,
        });

        (status, axum::Json(body)).into_response()
    }
}
