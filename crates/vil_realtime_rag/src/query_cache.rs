use std::collections::HashMap;
use parking_lot::RwLock;

/// Cache for pre-computed query embeddings.
///
/// Avoids embedding computation for repeated or common queries.
/// Uses a simple FNV-1a hash of the query string as the cache key.
pub struct QueryCache {
    cache: RwLock<HashMap<u64, Vec<f32>>>,
    max_entries: usize,
}

impl QueryCache {
    /// Create a new cache that holds at most `max_entries` embeddings.
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: RwLock::new(HashMap::with_capacity(max_entries.min(1024))),
            max_entries,
        }
    }

    /// Get cached embedding for a query string.
    pub fn get(&self, query: &str) -> Option<Vec<f32>> {
        let key = hash_query(query);
        self.cache.read().get(&key).cloned()
    }

    /// Store an embedding for a query string.
    ///
    /// If the cache is full, it is cleared before inserting (simple eviction).
    pub fn put(&self, query: &str, embedding: Vec<f32>) {
        let key = hash_query(query);
        let mut cache = self.cache.write();
        if cache.len() >= self.max_entries {
            cache.clear();
        }
        cache.insert(key, embedding);
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.cache.read().len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.read().is_empty()
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        self.cache.write().clear();
    }
}

/// Simple FNV-1a hash for query strings.
fn hash_query(query: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for byte in query.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_and_get() {
        let cache = QueryCache::new(100);
        let emb = vec![1.0, 2.0, 3.0];
        cache.put("hello", emb.clone());
        assert_eq!(cache.get("hello"), Some(emb));
    }

    #[test]
    fn cache_miss() {
        let cache = QueryCache::new(100);
        assert_eq!(cache.get("nonexistent"), None);
    }

    #[test]
    fn max_entries_eviction() {
        let cache = QueryCache::new(3);
        cache.put("a", vec![1.0]);
        cache.put("b", vec![2.0]);
        cache.put("c", vec![3.0]);
        assert_eq!(cache.len(), 3);

        // Inserting a 4th triggers clear + insert.
        cache.put("d", vec![4.0]);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get("d"), Some(vec![4.0]));
        assert_eq!(cache.get("a"), None);
    }

    #[test]
    fn len_and_clear() {
        let cache = QueryCache::new(100);
        assert!(cache.is_empty());
        cache.put("x", vec![0.0]);
        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn hash_deterministic() {
        assert_eq!(hash_query("test"), hash_query("test"));
        assert_ne!(hash_query("test"), hash_query("tset"));
    }
}
