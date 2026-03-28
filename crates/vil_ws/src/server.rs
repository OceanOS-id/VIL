// =============================================================================
// vil_ws::server — WsServer
// =============================================================================
//
// WebSocket server with VIL semantic log integration.
//
// - Every send emits mq_log! (op_type=0 publish) with timing.
// - Every receive emits mq_log! (op_type=1 consume) with timing.
// - No println!, tracing::info!, or log::info! — COMPLIANCE.md §8.
// - String fields use register_str() hashes — no raw strings on hot path.
//
// Thread hint: WsServer spawns 1 accept loop task.
// Add 1 to LogConfig.threads for optimal log ring sizing.
// =============================================================================

use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::{accept_async, tungstenite::Message};

use vil_log::dict::register_str;
use vil_log::{mq_log, types::MqPayload};

use crate::config::WsConfig;
use crate::error::WsFault;
use crate::room::{ClientSender, RoomManager};

/// Dedicated WebSocket server with integrated VIL semantic logging.
///
/// Every send emits `mq_log!` (op_type=0 publish) and every receive emits
/// `mq_log!` (op_type=1 consume) with wall-clock timing.
///
/// Thread hint: 1 accept task spawned on `start()`. Add 1 to `LogConfig.threads`.
pub struct WsServer {
    config: WsConfig,
    rooms: RoomManager,
    connection_count: Arc<AtomicUsize>,
    /// Cached FxHash of the bind address.
    addr_hash: u32,
}

impl WsServer {
    /// Create a new `WsServer` with the given config.
    pub fn new(config: WsConfig) -> Self {
        let addr_hash = register_str(&config.addr);
        Self {
            config,
            rooms: RoomManager::new(),
            connection_count: Arc::new(AtomicUsize::new(0)),
            addr_hash,
        }
    }

    /// Return a clone of the room manager for external room control.
    pub fn room_manager(&self) -> RoomManager {
        self.rooms.clone()
    }

    /// Return the current live connection count.
    pub fn connection_count(&self) -> usize {
        self.connection_count.load(Ordering::Relaxed)
    }

    /// Start accepting WebSocket connections.
    ///
    /// Binds the TCP listener and spawns the accept loop as a tokio task.
    /// Returns a `JoinHandle` that resolves when the server shuts down.
    pub async fn start(self: Arc<Self>) -> Result<tokio::task::JoinHandle<()>, WsFault> {
        let addr_hash = self.addr_hash;
        let listener =
            TcpListener::bind(&self.config.addr)
                .await
                .map_err(|e| WsFault::BindFailed {
                    addr_hash,
                    reason_code: e.raw_os_error().unwrap_or(0) as u32,
                })?;

        let server = self.clone();
        let handle = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _peer)) => {
                        let count = server.connection_count.load(Ordering::Relaxed);
                        if count >= server.config.max_connections {
                            // Reject silently — no mq_log for rejected connections
                            drop(stream);
                            continue;
                        }
                        let rooms = server.rooms.clone();
                        let max_bytes = server.config.max_message_bytes;
                        let addr_hash = server.addr_hash;
                        let conn_count = server.connection_count.clone();

                        conn_count.fetch_add(1, Ordering::Relaxed);
                        tokio::spawn(async move {
                            let _ = handle_connection(stream, rooms, max_bytes, addr_hash).await;
                            conn_count.fetch_sub(1, Ordering::Relaxed);
                        });
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(handle)
    }

    /// Broadcast a raw message to all connected clients.
    ///
    /// Emits `mq_log!` (op_type=0 publish) with timing.
    pub fn broadcast(&self, topic: &str, msg: &[u8]) -> u32 {
        let start = std::time::Instant::now();
        let topic_hash = register_str(topic);

        let sent = self.rooms.broadcast_all(msg);

        let elapsed = start.elapsed();
        mq_log!(
            Info,
            MqPayload {
                broker_hash: self.addr_hash,
                topic_hash,
                message_bytes: msg.len() as u32,
                e2e_latency_us: elapsed.as_micros() as u32,
                op_type: 0, // publish / send
                ..MqPayload::default()
            }
        );

        sent
    }

    /// Broadcast a raw message to all clients in a named room.
    ///
    /// Emits `mq_log!` (op_type=0 publish) with timing.
    pub fn broadcast_room(&self, room: &str, msg: &[u8]) -> Result<u32, WsFault> {
        let start = std::time::Instant::now();
        let topic_hash = register_str(room);

        let result = self.rooms.broadcast_room(room, msg);

        let elapsed = start.elapsed();
        let (_sent, _err_code) = match &result {
            Ok(n) => (*n, 0u8),
            Err(f) => (0, f.as_error_code()),
        };

        mq_log!(
            Info,
            MqPayload {
                broker_hash: self.addr_hash,
                topic_hash,
                message_bytes: msg.len() as u32,
                e2e_latency_us: elapsed.as_micros() as u32,
                op_type: 0, // publish / send
                ..MqPayload::default()
            }
        );

        result
    }
}

// =============================================================================
// Internal — per-connection handler task
// =============================================================================

async fn handle_connection(
    stream: TcpStream,
    rooms: RoomManager,
    max_bytes: usize,
    broker_hash: u32,
) -> Result<(), WsFault> {
    let ws_stream = accept_async(stream)
        .await
        .map_err(|_| WsFault::HandshakeFailed { reason_code: 1 })?;

    let (mut ws_sink, mut ws_source) = ws_stream.split();

    // Create an outbound channel for this connection
    let (tx, mut rx): (ClientSender, _) = mpsc::unbounded_channel();
    let client_id = rooms.register_client(tx);

    // Spawn outbound writer task
    let write_task = tokio::spawn(async move {
        while let Some(msg_bytes) = rx.recv().await {
            let start = std::time::Instant::now();
            let msg = Message::binary(msg_bytes.clone());
            let send_result = ws_sink.send(msg).await;
            let elapsed = start.elapsed();

            mq_log!(
                Info,
                MqPayload {
                    broker_hash,
                    topic_hash: 0,
                    message_bytes: msg_bytes.len() as u32,
                    e2e_latency_us: elapsed.as_micros() as u32,
                    op_type: 0, // publish / send
                    ..MqPayload::default()
                }
            );

            if send_result.is_err() {
                break;
            }
        }
    });

    // Inbound reader loop
    while let Some(msg_result) = ws_source.next().await {
        match msg_result {
            Ok(msg) => {
                let start = std::time::Instant::now();
                let msg_bytes = msg.into_data();

                if msg_bytes.len() > max_bytes {
                    rooms.remove_client(client_id);
                    write_task.abort();
                    return Err(WsFault::MessageTooLarge {
                        received_bytes: msg_bytes.len() as u32,
                        max_bytes: max_bytes as u32,
                    });
                }

                let elapsed = start.elapsed();
                mq_log!(
                    Info,
                    MqPayload {
                        broker_hash,
                        topic_hash: client_id as u32,
                        message_bytes: msg_bytes.len() as u32,
                        e2e_latency_us: elapsed.as_micros() as u32,
                        op_type: 1, // consume / receive
                        ..MqPayload::default()
                    }
                );
            }
            Err(_) => break,
        }
    }

    rooms.remove_client(client_id);
    write_task.abort();
    Ok(())
}
