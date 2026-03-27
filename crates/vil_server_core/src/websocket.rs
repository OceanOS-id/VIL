// =============================================================================
// VIL Server WebSocket — Upgrade support
// =============================================================================
//
// Provides WebSocket upgrade handling for vil-server handlers.
// Built on Axum's WebSocket support with convenience helpers.
//
// # Example
// ```no_run
// use vil_server_core::websocket::*;
// use axum::extract::ws::{WebSocket, Message};
//
// async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
//     ws.on_upgrade(handle_socket)
// }
//
// async fn handle_socket(mut socket: WebSocket) {
//     while let Some(Ok(msg)) = socket.recv().await {
//         if let Message::Text(text) = msg {
//             socket.send(Message::Text(format!("echo: {}", text))).await.ok();
//         }
//     }
// }
// ```

// Re-export Axum's WebSocket types for convenience
pub use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};

use axum::response::IntoResponse;
use std::time::Duration;

/// Configuration for WebSocket connections.
#[derive(Debug, Clone)]
pub struct WsConfig {
    /// Maximum message size in bytes (default: 64KB)
    pub max_message_size: usize,
    /// Ping interval (default: 30s)
    pub ping_interval: Duration,
    /// Close timeout — how long to wait for close frame (default: 5s)
    pub close_timeout: Duration,
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            max_message_size: 64 * 1024,
            ping_interval: Duration::from_secs(30),
            close_timeout: Duration::from_secs(5),
        }
    }
}

/// Helper to create a WebSocket echo handler (useful for testing).
pub async fn echo_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(echo_socket)
}

async fn echo_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                if socket.send(Message::Text(format!("echo: {}", text))).await.is_err() {
                    break;
                }
            }
            Message::Binary(data) => {
                if socket.send(Message::Binary(data)).await.is_err() {
                    break;
                }
            }
            Message::Ping(data) => {
                if socket.send(Message::Pong(data)).await.is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}

/// Helper to create a broadcast channel for WebSocket fan-out.
///
/// Returns (sender, receiver_factory) where:
/// - sender: send messages to all connected clients
/// - receiver_factory: call to get a new receiver for each new connection
pub fn broadcast_channel(
    capacity: usize,
) -> (
    tokio::sync::broadcast::Sender<String>,
    impl Fn() -> tokio::sync::broadcast::Receiver<String>,
) {
    let (tx, _) = tokio::sync::broadcast::channel(capacity);
    let tx_clone = tx.clone();
    let factory = move || tx_clone.subscribe();
    (tx, factory)
}
