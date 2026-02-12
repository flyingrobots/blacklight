use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;

use crate::server::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/ws", get(ws_handler))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.notifications.subscribe();

    loop {
        match rx.recv().await {
            Ok(notification) => {
                let json = match serde_json::to_string(&notification) {
                    Ok(j) => j,
                    Err(_) => continue,
                };
                if socket.send(Message::Text(json.into())).await.is_err() {
                    break; // Client disconnected
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                tracing::debug!("ws client lagged, skipped {n} notifications");
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                break; // Channel closed
            }
        }
    }
}
