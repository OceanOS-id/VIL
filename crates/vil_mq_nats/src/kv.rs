// =============================================================================
// NATS KV Store — Distributed key-value (real async-nats KV)
// =============================================================================

use bytes::Bytes;
use tokio::sync::broadcast;

/// KV entry.
#[derive(Debug, Clone)]
pub struct KvEntry {
    pub key: String,
    pub value: Bytes,
    pub revision: u64,
}

/// NATS KV Store backed by real async-nats JetStream KV.
pub struct KvStore {
    bucket: String,
    store: async_nats::jetstream::kv::Store,
    watch_tx: broadcast::Sender<KvEntry>,
}

impl KvStore {
    /// Create or open a KV bucket from a JetStream context.
    pub async fn new(js: &async_nats::jetstream::Context, bucket: &str) -> Result<Self, String> {
        let store = js.create_key_value(async_nats::jetstream::kv::Config {
            bucket: bucket.into(),
            ..Default::default()
        }).await.map_err(|e| format!("KV bucket '{}' creation failed: {}", bucket, e))?;

        let (tx, _) = broadcast::channel(256);
        tracing::info!(bucket = %bucket, "nats kv store opened");
        Ok(Self { bucket: bucket.into(), store, watch_tx: tx })
    }

    /// Put a key-value pair.
    pub async fn put(&self, key: &str, value: &[u8]) -> Result<u64, String> {
        let rev = self.store.put(key, Bytes::copy_from_slice(value)).await
            .map_err(|e| format!("KV put failed: {}", e))?;

        // Notify local watchers
        let bytes = Bytes::copy_from_slice(value);
        let _ = self.watch_tx.send(KvEntry {
            key: key.to_string(), value: bytes, revision: rev,
        });

        tracing::debug!(bucket = %self.bucket, key = %key, rev = rev, "kv put");
        Ok(rev)
    }

    /// Get a value by key.
    pub async fn get(&self, key: &str) -> Option<KvEntry> {
        match self.store.entry(key).await {
            Ok(Some(entry)) => Some(KvEntry {
                key: entry.key.clone(),
                value: entry.value.clone(),
                revision: entry.revision,
            }),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!(key = %key, error = %e, "kv get failed");
                None
            }
        }
    }

    /// Delete a key.
    pub async fn delete(&self, key: &str) -> bool {
        self.store.delete(key).await.is_ok()
    }

    /// List all keys.
    pub async fn keys(&self) -> Vec<String> {
        match self.store.keys().await {
            Ok(mut keys_stream) => {
                let mut result = Vec::new();
                use futures::StreamExt;
                while let Some(key) = keys_stream.next().await {
                    if let Ok(k) = key {
                        result.push(k);
                    }
                }
                result
            }
            Err(_) => Vec::new(),
        }
    }

    /// Watch for changes (returns a broadcast receiver for local notifications).
    pub fn watch(&self) -> broadcast::Receiver<KvEntry> {
        self.watch_tx.subscribe()
    }

    /// Get number of keys (note: requires listing all keys).
    pub async fn len(&self) -> usize {
        self.keys().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.keys().await.is_empty()
    }

    pub fn bucket(&self) -> &str { &self.bucket }

    /// Access the underlying async-nats KV store for advanced use cases.
    pub fn inner(&self) -> &async_nats::jetstream::kv::Store { &self.store }
}
