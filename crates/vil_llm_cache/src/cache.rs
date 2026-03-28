use std::time::{SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use parking_lot::RwLock;

use crate::config::CacheConfig;
use crate::hasher::hash_messages;
use crate::similarity::find_similar;

/// A cached LLM response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachedResponse {
    /// The response content.
    pub content: String,
    /// The model that generated this response.
    pub model: String,
    /// Unix timestamp (ms) when this entry was created.
    pub created_at: u64,
    /// Time-to-live in milliseconds.
    pub ttl_ms: u64,
    /// Number of times this entry has been returned as a hit.
    pub hit_count: u32,
}

impl CachedResponse {
    /// Returns true if this entry has expired.
    pub fn is_expired(&self) -> bool {
        let now = current_time_ms();
        now > self.created_at + self.ttl_ms
    }
}

/// Cache statistics.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub exact_hits: u64,
    pub semantic_hits: u64,
    pub misses: u64,
}

impl CacheStats {
    /// Overall hit rate as a fraction in [0.0, 1.0].
    pub fn hit_rate(&self) -> f64 {
        let total = self.exact_hits + self.semantic_hits + self.misses;
        if total == 0 {
            return 0.0;
        }
        (self.exact_hits + self.semantic_hits) as f64 / total as f64
    }
}

/// Semantic response cache with exact-match (FNV hash) and embedding-based
/// similarity lookup.
pub struct SemanticCache {
    /// Exact-match store keyed by FNV hash of serialized messages.
    exact: DashMap<u64, CachedResponse>,
    /// Embedding-based semantic index: (embedding_vector, CachedResponse).
    embeddings: RwLock<Vec<(Vec<f32>, CachedResponse)>>,
    /// Cache configuration.
    config: CacheConfig,
    /// Runtime statistics.
    stats: RwLock<CacheStats>,
}

impl SemanticCache {
    /// Create a new cache with the given configuration.
    pub fn new(config: CacheConfig) -> Self {
        Self {
            exact: DashMap::new(),
            embeddings: RwLock::new(Vec::new()),
            config,
            stats: RwLock::new(CacheStats::default()),
        }
    }

    /// Create a cache with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(CacheConfig::default())
    }

    /// Look up an exact match by hashing the serialized messages.
    pub fn get_exact(&self, messages: &str) -> Option<CachedResponse> {
        let key = hash_messages(messages);
        if let Some(mut entry) = self.exact.get_mut(&key) {
            if entry.is_expired() {
                drop(entry);
                self.exact.remove(&key);
                self.stats.write().misses += 1;
                return None;
            }
            entry.hit_count += 1;
            let resp = entry.clone();
            drop(entry);
            self.stats.write().exact_hits += 1;
            return Some(resp);
        }
        self.stats.write().misses += 1;
        None
    }

    /// Look up a semantically similar cached response using embedding similarity.
    pub fn get_similar(&self, query_embedding: &[f32], threshold: f32) -> Option<CachedResponse> {
        let embs = self.embeddings.read();
        if embs.is_empty() {
            self.stats.write().misses += 1;
            return None;
        }

        // Build candidate list filtering expired entries
        let candidates: Vec<(Vec<f32>, usize)> = embs
            .iter()
            .enumerate()
            .filter(|(_, (_, resp))| !resp.is_expired())
            .map(|(i, (emb, _))| (emb.clone(), i))
            .collect();

        if let Some((idx, _sim)) = find_similar(query_embedding, &candidates, threshold) {
            let resp = embs[idx].1.clone();
            drop(embs);
            // Increment hit count
            self.embeddings.write()[idx].1.hit_count += 1;
            self.stats.write().semantic_hits += 1;
            return Some(resp);
        }

        drop(embs);
        self.stats.write().misses += 1;
        None
    }

    /// Store a response in both exact and semantic indices.
    pub fn put(
        &self,
        messages: &str,
        query_embedding: Option<Vec<f32>>,
        content: String,
        model: String,
    ) {
        let now = current_time_ms();
        let resp = CachedResponse {
            content,
            model,
            created_at: now,
            ttl_ms: self.config.ttl_ms,
            hit_count: 0,
        };

        // Enforce max entries — evict oldest if at capacity
        if self.exact.len() >= self.config.max_entries {
            self.evict_oldest_exact();
        }

        let key = hash_messages(messages);
        self.exact.insert(key, resp.clone());

        if let Some(embedding) = query_embedding {
            let mut embs = self.embeddings.write();
            if embs.len() >= self.config.max_entries {
                // Remove oldest (first) entry
                embs.remove(0);
            }
            embs.push((embedding, resp));
        }
    }

    /// Remove all expired entries from both stores.
    pub fn evict_expired(&self) -> usize {
        let mut removed = 0;

        // Exact store
        let keys_to_remove: Vec<u64> = self
            .exact
            .iter()
            .filter(|entry| entry.value().is_expired())
            .map(|entry| *entry.key())
            .collect();
        for key in &keys_to_remove {
            self.exact.remove(key);
            removed += 1;
        }

        // Embedding store
        let mut embs = self.embeddings.write();
        let before = embs.len();
        embs.retain(|(_, resp)| !resp.is_expired());
        removed += before - embs.len();

        removed
    }

    /// Get current cache statistics.
    pub fn stats(&self) -> CacheStats {
        let mut s = self.stats.read().clone();
        s.total_entries = self.exact.len() + self.embeddings.read().len();
        s
    }

    /// Total number of entries across both stores.
    pub fn len(&self) -> usize {
        self.exact.len() + self.embeddings.read().len()
    }

    /// Returns true if both stores are empty.
    pub fn is_empty(&self) -> bool {
        self.exact.is_empty() && self.embeddings.read().is_empty()
    }

    fn evict_oldest_exact(&self) {
        // Find the entry with the smallest created_at
        if let Some(oldest_key) = self
            .exact
            .iter()
            .min_by_key(|entry| entry.value().created_at)
            .map(|entry| *entry.key())
        {
            self.exact.remove(&oldest_key);
        }
    }
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(max: usize, ttl_ms: u64) -> CacheConfig {
        CacheConfig {
            max_entries: max,
            ttl_ms,
            similarity_threshold: 0.9,
        }
    }

    #[test]
    fn exact_cache_hit() {
        let cache = SemanticCache::new(make_config(100, 60_000));
        cache.put("hello", None, "world".into(), "gpt-4".into());

        let hit = cache.get_exact("hello");
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().content, "world");
    }

    #[test]
    fn exact_cache_miss() {
        let cache = SemanticCache::new(make_config(100, 60_000));
        cache.put("hello", None, "world".into(), "gpt-4".into());

        let miss = cache.get_exact("goodbye");
        assert!(miss.is_none());
    }

    #[test]
    fn semantic_similarity_hit() {
        let cache = SemanticCache::new(make_config(100, 60_000));
        let emb = vec![1.0, 0.0, 0.0];
        cache.put("q1", Some(emb.clone()), "answer1".into(), "gpt-4".into());

        // Query with identical embedding
        let hit = cache.get_similar(&emb, 0.9);
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().content, "answer1");
    }

    #[test]
    fn semantic_similarity_below_threshold_miss() {
        let cache = SemanticCache::new(make_config(100, 60_000));
        cache.put("q1", Some(vec![1.0, 0.0]), "answer1".into(), "gpt-4".into());

        // Orthogonal vector = cosine similarity 0.0
        let miss = cache.get_similar(&[0.0, 1.0], 0.5);
        assert!(miss.is_none());
    }

    #[test]
    fn ttl_expiry() {
        let cache = SemanticCache::new(make_config(100, 0)); // TTL = 0ms, expires immediately
        cache.put(
            "hello",
            Some(vec![1.0, 0.0]),
            "world".into(),
            "gpt-4".into(),
        );

        // Entries should be expired
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(cache.get_exact("hello").is_none());
        assert!(cache.get_similar(&[1.0, 0.0], 0.9).is_none());
    }

    #[test]
    fn evict_expired_removes_entries() {
        let cache = SemanticCache::new(make_config(100, 0));
        cache.put("a", Some(vec![1.0]), "resp_a".into(), "m".into());
        cache.put("b", Some(vec![0.0]), "resp_b".into(), "m".into());

        std::thread::sleep(std::time::Duration::from_millis(5));
        let removed = cache.evict_expired();
        assert!(removed >= 2);
        assert!(cache.is_empty());
    }

    #[test]
    fn stats_tracking() {
        let cache = SemanticCache::new(make_config(100, 60_000));
        cache.put("q", None, "a".into(), "m".into());
        cache.get_exact("q"); // hit
        cache.get_exact("missing"); // miss

        let s = cache.stats();
        assert_eq!(s.exact_hits, 1);
        assert_eq!(s.misses, 1);
        assert!(s.hit_rate() > 0.0);
    }

    #[test]
    fn max_entries_enforced() {
        let cache = SemanticCache::new(make_config(2, 60_000));
        cache.put("a", Some(vec![1.0]), "r1".into(), "m".into());
        cache.put("b", Some(vec![0.0]), "r2".into(), "m".into());
        cache.put("c", Some(vec![0.5]), "r3".into(), "m".into());

        // Exact store should have at most 2 entries
        assert!(cache.exact.len() <= 2);
    }

    #[test]
    fn put_get_roundtrip() {
        let cache = SemanticCache::new(make_config(100, 60_000));
        cache.put("test-msg", None, "test-content".into(), "gpt-4o".into());

        let resp = cache.get_exact("test-msg").unwrap();
        assert_eq!(resp.content, "test-content");
        assert_eq!(resp.model, "gpt-4o");
        assert_eq!(resp.hit_count, 1);
    }

    #[test]
    fn empty_cache_returns_none() {
        let cache = SemanticCache::new(make_config(100, 60_000));
        assert!(cache.get_exact("anything").is_none());
        assert!(cache.get_similar(&[1.0, 0.0], 0.5).is_none());
        assert!(cache.is_empty());
    }

    #[test]
    fn hit_rate_zero_when_empty() {
        let s = CacheStats::default();
        assert_eq!(s.hit_rate(), 0.0);
    }

    #[test]
    fn semantic_hit_increments_counter() {
        let cache = SemanticCache::new(make_config(100, 60_000));
        let emb = vec![1.0, 0.0];
        cache.put("q", Some(emb.clone()), "a".into(), "m".into());
        cache.get_similar(&emb, 0.9);
        cache.get_similar(&emb, 0.9);

        let s = cache.stats();
        assert_eq!(s.semantic_hits, 2);
    }
}
