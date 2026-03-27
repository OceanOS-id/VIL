// =============================================================================
// VIL Server Mesh — Event Bus (In-Process Pub/Sub)
// =============================================================================
//
// Publish/subscribe event system for services within a process monolith.
// Events are delivered via broadcast channels — all subscribers receive every event.
//
// Uses tokio::sync::broadcast for zero-copy event fan-out within the process.
// For cross-process events, use the Tri-Lane mesh or MQ adapter.

use bytes::Bytes;
use dashmap::DashMap;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};

/// An event published to the bus.
#[derive(Debug, Clone)]
pub struct Event {
    /// Event type/topic
    pub topic: String,
    /// Event payload (JSON bytes)
    pub payload: Bytes,
    /// Publisher service name
    pub source: String,
    /// Timestamp (unix secs)
    pub timestamp: u64,
    /// Monotonic event ID
    pub id: u64,
}

/// Event bus — in-process pub/sub.
pub struct EventBus {
    /// Topic → broadcast sender
    topics: DashMap<String, tokio::sync::broadcast::Sender<Event>>,
    /// Channel capacity per topic
    capacity: usize,
    /// Total events published
    total_published: AtomicU64,
    /// Next event ID
    next_id: AtomicU64,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        Self {
            topics: DashMap::new(),
            capacity,
            total_published: AtomicU64::new(0),
            next_id: AtomicU64::new(1),
        }
    }

    /// Get or create a topic channel.
    fn get_or_create_topic(&self, topic: &str) -> tokio::sync::broadcast::Sender<Event> {
        self.topics
            .entry(topic.to_string())
            .or_insert_with(|| {
                let (tx, _) = tokio::sync::broadcast::channel(self.capacity);
                tx
            })
            .clone()
    }

    /// Publish an event to a topic.
    pub fn publish(&self, topic: &str, source: &str, payload: impl Into<Bytes>) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let event = Event {
            topic: topic.to_string(),
            payload: payload.into(),
            source: source.to_string(),
            timestamp,
            id,
        };

        let tx = self.get_or_create_topic(topic);
        let _ = tx.send(event); // Ignore if no subscribers
        self.total_published.fetch_add(1, Ordering::Relaxed);
    }

    /// Publish a JSON-serializable event.
    pub fn publish_json<T: Serialize>(&self, topic: &str, source: &str, data: &T) {
        let bytes = serde_json::to_vec(data).unwrap_or_default();
        self.publish(topic, source, bytes);
    }

    /// Subscribe to a topic. Returns a receiver.
    pub fn subscribe(&self, topic: &str) -> tokio::sync::broadcast::Receiver<Event> {
        let tx = self.get_or_create_topic(topic);
        tx.subscribe()
    }

    /// Get total published events.
    pub fn total_published(&self) -> u64 {
        self.total_published.load(Ordering::Relaxed)
    }

    /// Get active topic count.
    pub fn topic_count(&self) -> usize {
        self.topics.len()
    }

    /// List active topics.
    pub fn topics(&self) -> Vec<String> {
        self.topics.iter().map(|e| e.key().clone()).collect()
    }

    /// Get subscriber count for a topic.
    pub fn subscriber_count(&self, topic: &str) -> usize {
        self.topics.get(topic)
            .map(|tx| tx.receiver_count())
            .unwrap_or(0)
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}
