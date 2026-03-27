/// Configuration for the semantic cache.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheConfig {
    /// Maximum number of entries in the cache.
    pub max_entries: usize,
    /// Default time-to-live in milliseconds for cached responses.
    pub ttl_ms: u64,
    /// Cosine similarity threshold for semantic matching (0.0 to 1.0).
    pub similarity_threshold: f32,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            ttl_ms: 3_600_000, // 1 hour
            similarity_threshold: 0.92,
        }
    }
}
