// =============================================================================
// VIL Server — SHM Query Cache
// =============================================================================
//
// Caches database query results in SHM ExchangeHeap regions.
// All co-located services can read cached results via zero-copy.
//
// Flow:
//   1. Handler issues DB query
//   2. Result serialized to JSON bytes
//   3. JSON written to SHM region
//   4. Cache entry stores region_id + offset + len
//   5. Subsequent requests read from SHM (0 copy)
//   6. TTL-based invalidation

use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use vil_shm::{ExchangeHeap, Offset};
use vil_types::RegionId;

/// SHM-backed query cache entry.
struct CacheEntry {
    region_id: RegionId,
    offset: Offset,
    len: usize,
    cached_at: Instant,
    ttl: Duration,
    hit_count: u64,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }
}

/// Query cache backed by SHM ExchangeHeap.
///
/// All cached data lives in shared memory — readable by all
/// services in the process monolith without copying.
pub struct ShmQueryCache {
    heap: Arc<ExchangeHeap>,
    region_id: RegionId,
    entries: DashMap<String, CacheEntry>,
    default_ttl: Duration,
    max_entries: usize,
    hits: std::sync::atomic::AtomicU64,
    misses: std::sync::atomic::AtomicU64,
}

impl ShmQueryCache {
    /// Create a new SHM query cache.
    ///
    /// - `region_size`: SHM region capacity (default: 32MB)
    /// - `default_ttl`: cache entry TTL (default: 60s)
    /// - `max_entries`: max cached queries (default: 10000)
    pub fn new(
        heap: Arc<ExchangeHeap>,
        region_size: usize,
        default_ttl: Duration,
        max_entries: usize,
    ) -> Self {
        let region_id = heap.create_region("vil_query_cache", region_size);
        Self {
            heap, region_id, entries: DashMap::new(),
            default_ttl, max_entries,
            hits: std::sync::atomic::AtomicU64::new(0),
            misses: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Default: 32MB region, 60s TTL, 10K max entries.
    pub fn default_cache(heap: Arc<ExchangeHeap>) -> Self {
        Self::new(heap, 32 * 1024 * 1024, Duration::from_secs(60), 10000)
    }

    /// Get a cached query result.
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let mut entry = self.entries.get_mut(key)?;
        if entry.is_expired() {
            drop(entry);
            self.entries.remove(key);
            self.misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return None;
        }
        entry.hit_count += 1;
        self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.heap.read_bytes(entry.region_id, entry.offset, entry.len)
    }

    /// Get cached result as JSON.
    pub fn get_json<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let bytes = self.get(key)?;
        serde_json::from_slice(&bytes).ok()
    }

    /// Cache a query result.
    pub fn put(&self, key: &str, data: &[u8]) -> bool {
        self.put_with_ttl(key, data, self.default_ttl)
    }

    /// Cache with custom TTL.
    pub fn put_with_ttl(&self, key: &str, data: &[u8], ttl: Duration) -> bool {
        if self.entries.len() >= self.max_entries {
            self.evict_expired();
        }

        let offset = match self.heap.alloc_bytes(self.region_id, data.len(), 8) {
            Some(o) => o,
            None => {
                // Region full — reset and retry
                self.heap.reset_region(self.region_id);
                self.entries.clear();
                match self.heap.alloc_bytes(self.region_id, data.len(), 8) {
                    Some(o) => o,
                    None => return false,
                }
            }
        };

        if !self.heap.write_bytes(self.region_id, offset, data) {
            return false;
        }

        self.entries.insert(key.to_string(), CacheEntry {
            region_id: self.region_id,
            offset,
            len: data.len(),
            cached_at: Instant::now(),
            ttl,
            hit_count: 0,
        });

        true
    }

    /// Cache JSON-serializable data.
    pub fn put_json<T: serde::Serialize>(&self, key: &str, data: &T) -> bool {
        match serde_json::to_vec(data) {
            Ok(bytes) => self.put(key, &bytes),
            Err(_) => false,
        }
    }

    /// Invalidate a cache entry.
    pub fn invalidate(&self, key: &str) {
        self.entries.remove(key);
    }

    /// Invalidate all entries matching a prefix.
    pub fn invalidate_prefix(&self, prefix: &str) {
        self.entries.retain(|k, _| !k.starts_with(prefix));
    }

    /// Evict expired entries.
    pub fn evict_expired(&self) -> usize {
        let before = self.entries.len();
        self.entries.retain(|_, entry| !entry.is_expired());
        before - self.entries.len()
    }

    /// Get cache statistics.
    pub fn stats(&self) -> QueryCacheStats {
        let hits = self.hits.load(std::sync::atomic::Ordering::Relaxed);
        let misses = self.misses.load(std::sync::atomic::Ordering::Relaxed);
        let total = hits + misses;
        let (used, capacity) = self.heap.region_stats(self.region_id)
            .map(|s| (s.used, s.capacity))
            .unwrap_or((0, 0));
        QueryCacheStats {
            entries: self.entries.len(),
            max_entries: self.max_entries,
            hits, misses,
            hit_rate: if total > 0 { hits as f64 / total as f64 } else { 0.0 },
            shm_used_bytes: used,
            shm_capacity_bytes: capacity,
        }
    }
}

/// Query cache statistics.
#[derive(Debug, serde::Serialize)]
pub struct QueryCacheStats {
    pub entries: usize,
    pub max_entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub shm_used_bytes: usize,
    pub shm_capacity_bytes: usize,
}
