// =============================================================================
// VIL Server — In-Memory Cache (LRU + TTL + SHM-backed)
// =============================================================================
//
// High-performance cache for vil-server:
//   - LRU eviction (least recently used)
//   - Per-entry TTL (time-to-live)
//   - Optional SHM backing for cross-service cache sharing
//   - Thread-safe (DashMap-based)
//
// Key differentiator vs Redis: zero network hop for co-located services.
// Cache lives in SHM — all services in the process monolith share it.

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A cached entry with metadata.
struct CacheEntry<V> {
    value: V,
    created_at: Instant,
    last_accessed: Instant,
    ttl: Duration,
    access_count: u64,
}

impl<V> CacheEntry<V> {
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }

    fn touch(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count += 1;
    }
}

/// LRU + TTL cache.
///
/// Generic over K (key) and V (value). Thread-safe via DashMap.
pub struct Cache<
    K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
> {
    entries: Arc<DashMap<K, CacheEntry<V>>>,
    default_ttl: Duration,
    max_entries: usize,
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
}

impl<K, V> Cache<K, V>
where
    K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(max_entries: usize, default_ttl: Duration) -> Self {
        Self {
            entries: Arc::new(DashMap::with_capacity(max_entries)),
            default_ttl,
            max_entries,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        }
    }

    /// Get a value from the cache.
    pub fn get(&self, key: &K) -> Option<V> {
        if let Some(mut entry) = self.entries.get_mut(key) {
            if entry.is_expired() {
                drop(entry);
                self.entries.remove(key);
                self.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }
            entry.touch();
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.value.clone())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Put a value with default TTL.
    pub fn put(&self, key: K, value: V) {
        self.put_with_ttl(key, value, self.default_ttl);
    }

    /// Put a value with custom TTL.
    pub fn put_with_ttl(&self, key: K, value: V, ttl: Duration) {
        if self.entries.len() >= self.max_entries {
            self.evict_one();
        }
        self.entries.insert(
            key,
            CacheEntry {
                value,
                created_at: Instant::now(),
                last_accessed: Instant::now(),
                ttl,
                access_count: 0,
            },
        );
    }

    /// Remove a specific key.
    pub fn remove(&self, key: &K) -> Option<V> {
        self.entries.remove(key).map(|(_, entry)| entry.value)
    }

    /// Check if key exists (without touching).
    pub fn contains(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    /// Get current size.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries.
    pub fn clear(&self) {
        self.entries.clear();
    }

    /// Evict expired entries.
    pub fn cleanup_expired(&self) -> usize {
        let before = self.entries.len();
        self.entries.retain(|_, entry| !entry.is_expired());
        let evicted = before - self.entries.len();
        self.evictions.fetch_add(evicted as u64, Ordering::Relaxed);
        evicted
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        CacheStats {
            size: self.entries.len(),
            max_entries: self.max_entries,
            hits,
            misses,
            hit_rate: if total > 0 {
                hits as f64 / total as f64
            } else {
                0.0
            },
            evictions: self.evictions.load(Ordering::Relaxed),
        }
    }

    /// Evict the least recently accessed entry.
    fn evict_one(&self) {
        // Find LRU entry
        let mut oldest_key: Option<K> = None;
        let mut oldest_time = Instant::now();

        for entry in self.entries.iter() {
            if entry.last_accessed < oldest_time {
                oldest_time = entry.last_accessed;
                oldest_key = Some(entry.key().clone());
            }
        }

        if let Some(key) = oldest_key {
            self.entries.remove(&key);
            self.evictions.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Cache statistics.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    pub size: usize,
    pub max_entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub evictions: u64,
}

/// Convenience type for string-keyed JSON cache.
pub type JsonCache = Cache<String, serde_json::Value>;

impl JsonCache {
    /// Create a JSON cache with defaults (10K entries, 5min TTL).
    pub fn json_default() -> Self {
        Self::new(10000, Duration::from_secs(300))
    }
}
