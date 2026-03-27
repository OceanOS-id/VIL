// =============================================================================
// VIL Server Mesh — Dead Letter Queue
// =============================================================================
//
// Failed messages that cannot be processed are routed to the DLQ
// instead of being dropped. This enables:
//   - Post-mortem analysis of failures
//   - Manual replay of failed messages
//   - Alerting on DLQ growth

use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use crate::Lane;

/// A dead letter — a message that failed processing.
#[derive(Debug, Clone, Serialize)]
pub struct DeadLetter {
    pub id: u64,
    pub timestamp: u64,
    pub from_service: String,
    pub to_service: String,
    pub lane: String,
    pub payload_size: usize,
    pub error: String,
    pub retry_count: u32,
    pub original_timestamp: Option<u64>,
}

/// Dead letter queue.
pub struct DeadLetterQueue {
    letters: std::sync::RwLock<Vec<DeadLetter>>,
    max_size: usize,
    total_enqueued: AtomicU64,
    total_replayed: AtomicU64,
    next_id: AtomicU64,
}

impl DeadLetterQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            letters: std::sync::RwLock::new(Vec::new()),
            max_size,
            total_enqueued: AtomicU64::new(0),
            total_replayed: AtomicU64::new(0),
            next_id: AtomicU64::new(1),
        }
    }

    /// Enqueue a failed message.
    pub fn enqueue(
        &self,
        from: &str,
        to: &str,
        lane: Lane,
        payload_size: usize,
        error: &str,
        retry_count: u32,
    ) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let letter = DeadLetter {
            id,
            timestamp,
            from_service: from.to_string(),
            to_service: to.to_string(),
            lane: lane.to_string(),
            payload_size,
            error: error.to_string(),
            retry_count,
            original_timestamp: None,
        };

        self.total_enqueued.fetch_add(1, Ordering::Relaxed);

        let mut letters = self.letters.write().unwrap();
        if letters.len() >= self.max_size {
            letters.remove(0); // Evict oldest
        }
        letters.push(letter);

        tracing::warn!(
            from = %from,
            to = %to,
            lane = %lane,
            error = %error,
            "message sent to dead letter queue"
        );
    }

    /// Get recent dead letters.
    pub fn recent(&self, limit: usize) -> Vec<DeadLetter> {
        let letters = self.letters.read().unwrap();
        let start = letters.len().saturating_sub(limit);
        letters[start..].to_vec()
    }

    /// Get dead letter by ID.
    pub fn get(&self, id: u64) -> Option<DeadLetter> {
        let letters = self.letters.read().unwrap();
        letters.iter().find(|l| l.id == id).cloned()
    }

    /// Mark a dead letter as replayed.
    pub fn mark_replayed(&self, _id: u64) {
        self.total_replayed.fetch_add(1, Ordering::Relaxed);
    }

    /// Get queue depth (current size).
    pub fn depth(&self) -> usize {
        self.letters.read().unwrap().len()
    }

    /// Get total messages ever enqueued.
    pub fn total_enqueued(&self) -> u64 {
        self.total_enqueued.load(Ordering::Relaxed)
    }

    /// Get total messages replayed.
    pub fn total_replayed(&self) -> u64 {
        self.total_replayed.load(Ordering::Relaxed)
    }

    /// Clear the queue.
    pub fn clear(&self) {
        self.letters.write().unwrap().clear();
    }
}

impl Default for DeadLetterQueue {
    fn default() -> Self {
        Self::new(10000)
    }
}
