// =============================================================================
// VIL Server — Streaming Response & SSE Fan-Out Hub
// =============================================================================
//
// Chunked transfer streaming and multi-client SSE fan-out.

use bytes::Bytes;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::broadcast;

/// SSE fan-out hub — broadcasts events to all connected SSE clients.
///
/// Usage:
///   let hub = SseHub::new(1024);
///   // In handler: hub.subscribe() → SSE stream
///   // In background: hub.broadcast("topic", data)
pub struct SseHub {
    tx: broadcast::Sender<SseEvent>,
    connected_clients: AtomicU64,
    total_events: AtomicU64,
}

/// An SSE event with topic.
#[derive(Debug, Clone)]
pub struct SseEvent {
    pub topic: String,
    pub data: String,
    pub id: Option<String>,
}

impl SseHub {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            tx,
            connected_clients: AtomicU64::new(0),
            total_events: AtomicU64::new(0),
        }
    }

    /// Broadcast an event to all connected clients.
    pub fn broadcast(&self, topic: &str, data: impl Into<String>) {
        let event = SseEvent {
            topic: topic.to_string(),
            data: data.into(),
            id: None,
        };
        let _ = self.tx.send(event);
        self.total_events.fetch_add(1, Ordering::Relaxed);
    }

    /// Broadcast a JSON event.
    pub fn broadcast_json<T: serde::Serialize>(&self, topic: &str, data: &T) {
        let json = serde_json::to_string(data).unwrap_or_default();
        self.broadcast(topic, json);
    }

    /// Subscribe to the hub. Returns a receiver for SSE streaming.
    pub fn subscribe(&self) -> broadcast::Receiver<SseEvent> {
        self.connected_clients.fetch_add(1, Ordering::Relaxed);
        self.tx.subscribe()
    }

    /// Record client disconnect.
    pub fn client_disconnected(&self) {
        self.connected_clients.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get number of connected clients.
    pub fn connected_clients(&self) -> u64 {
        self.connected_clients.load(Ordering::Relaxed)
    }

    /// Get total events broadcast.
    pub fn total_events(&self) -> u64 {
        self.total_events.load(Ordering::Relaxed)
    }
}

impl Default for SseHub {
    fn default() -> Self {
        Self::new(1024)
    }
}

/// Chunked streaming body builder.
///
/// For streaming large responses without buffering the entire body.
pub struct StreamingBody {
    chunks: Vec<Bytes>,
}

impl StreamingBody {
    pub fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    pub fn chunk(mut self, data: impl Into<Bytes>) -> Self {
        self.chunks.push(data.into());
        self
    }

    pub fn chunks(&self) -> &[Bytes] {
        &self.chunks
    }

    pub fn total_size(&self) -> usize {
        self.chunks.iter().map(|c| c.len()).sum()
    }
}

impl Default for StreamingBody {
    fn default() -> Self { Self::new() }
}

// =============================================================================
// WsHub — WebSocket broadcast hub with topic-based routing
// =============================================================================

/// WebSocket broadcast hub with topic-based routing.
///
/// Similar to [`SseHub`] but designed for bidirectional WebSocket connections.
/// Each subscriber receives messages for a specific topic via an unbounded
/// MPSC channel.
///
/// # Example
///
/// ```no_run
/// use vil_server_core::streaming::WsHub;
///
/// let hub = WsHub::new();
/// let mut rx = hub.subscribe("chat");
/// hub.broadcast("chat", "hello".to_string());
/// // rx.recv().await => Some("hello")
/// ```
pub struct WsHub {
    /// topic -> list of sender channels
    channels: dashmap::DashMap<String, Vec<tokio::sync::mpsc::UnboundedSender<String>>>,
}

impl WsHub {
    /// Create a new empty WsHub.
    pub fn new() -> Self {
        Self {
            channels: dashmap::DashMap::new(),
        }
    }

    /// Subscribe to a topic. Returns a receiver for incoming messages.
    ///
    /// The returned receiver will yield all messages broadcast to the given
    /// topic until either the hub is dropped or the sender is cleaned up
    /// after the receiver is dropped.
    pub fn subscribe(&self, topic: &str) -> tokio::sync::mpsc::UnboundedReceiver<String> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        self.channels
            .entry(topic.to_string())
            .or_default()
            .push(tx);
        rx
    }

    /// Broadcast a message to all subscribers of a topic.
    ///
    /// Subscribers whose channels have been closed (receiver dropped) are
    /// automatically removed.
    pub fn broadcast(&self, topic: &str, message: String) {
        if let Some(mut senders) = self.channels.get_mut(topic) {
            senders.retain(|tx| tx.send(message.clone()).is_ok());
        }
    }

    /// Broadcast a JSON-serializable value to all subscribers of a topic.
    pub fn broadcast_json<T: serde::Serialize>(&self, topic: &str, data: &T) {
        let json = serde_json::to_string(data).unwrap_or_default();
        self.broadcast(topic, json);
    }

    /// Get the subscriber count for a specific topic.
    pub fn subscriber_count(&self, topic: &str) -> usize {
        self.channels.get(topic).map(|s| s.len()).unwrap_or(0)
    }

    /// Get the total subscriber count across all topics.
    pub fn total_subscribers(&self) -> usize {
        self.channels.iter().map(|e| e.value().len()).sum()
    }

    /// Get the number of active topics.
    pub fn topic_count(&self) -> usize {
        self.channels.len()
    }
}

impl Default for WsHub {
    fn default() -> Self {
        Self::new()
    }
}
