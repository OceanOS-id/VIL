// =============================================================================
// VIL Server — Idempotency Middleware (Request Deduplication)
// =============================================================================
//
// Prevents duplicate request processing using an Idempotency-Key header.
// If the same key is seen within the TTL window, the cached response
// is returned without re-executing the handler.
//
// Header: Idempotency-Key: <unique-key>
//
// Usage:
//   POST /api/payments  (with Idempotency-Key: pay-123)
//   → First time: execute handler, cache response
//   → Second time: return cached response (no re-execution)

use axum::body::Bytes;
use axum::http::StatusCode;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Cached response for idempotent requests.
#[derive(Clone)]
struct CachedResponse {
    status: StatusCode,
    body: Bytes,
    content_type: String,
    cached_at: Instant,
}

/// Idempotency store — holds cached responses keyed by Idempotency-Key.
pub struct IdempotencyStore {
    cache: Arc<DashMap<String, CachedResponse>>,
    /// Time-to-live for cached responses
    ttl: Duration,
    /// Maximum number of cached entries
    max_entries: usize,
}

impl IdempotencyStore {
    /// Create a new idempotency store.
    ///
    /// - `ttl`: how long to cache responses (default: 24 hours)
    /// - `max_entries`: maximum cache size (default: 10000)
    pub fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            ttl,
            max_entries,
        }
    }

    /// Default: 24h TTL, 10K max entries.
    pub fn default_config() -> Self {
        Self::new(Duration::from_secs(86400), 10000)
    }

    /// Check if a key has a cached response.
    pub fn get(&self, key: &str) -> Option<(StatusCode, Bytes, String)> {
        if let Some(entry) = self.cache.get(key) {
            if entry.cached_at.elapsed() < self.ttl {
                return Some((
                    entry.status,
                    entry.body.clone(),
                    entry.content_type.clone(),
                ));
            }
            // Expired — remove
            drop(entry);
            self.cache.remove(key);
        }
        None
    }

    /// Cache a response for a key.
    pub fn put(&self, key: String, status: StatusCode, body: Bytes, content_type: String) {
        // Evict expired entries if at capacity
        if self.cache.len() >= self.max_entries {
            self.evict_expired();
        }

        self.cache.insert(key, CachedResponse {
            status,
            body,
            content_type,
            cached_at: Instant::now(),
        });
    }

    /// Check if a key exists (without returning the cached data).
    pub fn contains(&self, key: &str) -> bool {
        self.cache.contains_key(key)
    }

    /// Get the number of cached entries.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Remove expired entries.
    pub fn evict_expired(&self) {
        let ttl = self.ttl;
        self.cache.retain(|_, v| v.cached_at.elapsed() < ttl);
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        self.cache.clear();
    }
}

impl Default for IdempotencyStore {
    fn default() -> Self {
        Self::default_config()
    }
}
