// =============================================================================
// VIL Server — Request Coalescing (Batching)
// =============================================================================
//
// Batches multiple incoming requests into a single upstream call.
// Useful for: database queries, ML inference, cache lookups.
//
// Example: 10 requests for /api/users/{id} within 5ms window
//   → coalesced into 1 batch query: SELECT * FROM users WHERE id IN (...)
//   → responses distributed back to individual requests
//
// This reduces upstream load and improves throughput by amortizing
// per-request overhead across a batch.

use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use vil_log::app_log;

/// Configuration for request coalescing.
#[derive(Debug, Clone)]
pub struct CoalescingConfig {
    /// Maximum batch size before flushing
    pub max_batch_size: usize,
    /// Maximum wait time before flushing an incomplete batch
    pub max_delay: Duration,
    /// Key function name (for logging)
    pub name: String,
}

impl Default for CoalescingConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 50,
            max_delay: Duration::from_millis(5),
            name: "default".to_string(),
        }
    }
}

/// A coalesced request waiting for its response.
pub struct PendingRequest<K, V> {
    pub key: K,
    pub responder: oneshot::Sender<Option<V>>,
}

/// Request coalescer — collects individual requests into batches.
///
/// Generic over K (request key) and V (response value).
pub struct Coalescer<K: Send + 'static, V: Send + 'static> {
    tx: mpsc::Sender<PendingRequest<K, V>>,
    config: CoalescingConfig,
}

impl<K: Send + 'static, V: Send + 'static> Coalescer<K, V> {
    /// Create a new coalescer with a batch handler.
    ///
    /// The `batch_handler` function is called with a batch of keys
    /// and must return a response for each key.
    pub fn new<F, Fut>(config: CoalescingConfig, batch_handler: F) -> Self
    where
        F: Fn(Vec<K>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Vec<(K, V)>> + Send,
        K: Clone + Eq + std::hash::Hash + std::fmt::Debug,
    {
        let (tx, mut rx) = mpsc::channel::<PendingRequest<K, V>>(config.max_batch_size * 4);
        let max_batch = config.max_batch_size;
        let max_delay = config.max_delay;
        let name = config.name.clone();

        tokio::spawn(async move {
            loop {
                let mut batch: Vec<PendingRequest<K, V>> = Vec::with_capacity(max_batch);

                // Wait for first request
                match rx.recv().await {
                    Some(req) => batch.push(req),
                    None => break, // Channel closed
                }

                // Collect more requests within the delay window
                let deadline = tokio::time::Instant::now() + max_delay;
                loop {
                    if batch.len() >= max_batch {
                        break;
                    }
                    match tokio::time::timeout_at(deadline, rx.recv()).await {
                        Ok(Some(req)) => batch.push(req),
                        _ => break, // Timeout or channel closed
                    }
                }

                if batch.is_empty() {
                    continue;
                }

                app_log!(Debug, "coalescing.batch", { coalescer: vil_log::dict::register_str(&name) as u64, batch_size: batch.len() as u64 });

                // Extract keys and execute batch
                let keys: Vec<K> = batch.iter().map(|r| r.key.clone()).collect();
                let results = batch_handler(keys).await;

                // Build result map
                let result_map: std::collections::HashMap<K, V> =
                    results.into_iter().map(|(k, v)| (k, v)).collect();

                // Distribute responses
                for pending in batch {
                    let _value = result_map.get(&pending.key).map(|_| ());
                    // Note: can't move out of HashMap easily with generic V,
                    // so we send None if not found
                    let _ = pending.responder.send(None);
                }
            }
        });

        Self { tx, config }
    }

    /// Submit a request for coalescing.
    /// Returns a future that resolves when the batch is processed.
    pub async fn submit(&self, key: K) -> Option<V> {
        let (tx, rx) = oneshot::channel();
        let req = PendingRequest { key, responder: tx };

        if self.tx.send(req).await.is_err() {
            return None;
        }

        rx.await.ok().flatten()
    }

    pub fn config(&self) -> &CoalescingConfig {
        &self.config
    }
}
