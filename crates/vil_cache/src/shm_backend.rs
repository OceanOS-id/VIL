use crate::cache_trait::VilCache;
use std::time::Duration;

/// SHM-backed cache using vil_server_core::ShmQueryCache.
/// Zero-copy for co-located services.
pub struct ShmCacheBackend {
    cache: vil_server_core::shm_query_cache::ShmQueryCache,
}

impl ShmCacheBackend {
    pub fn new(cache: vil_server_core::shm_query_cache::ShmQueryCache) -> Self {
        Self { cache }
    }
}

#[async_trait::async_trait]
impl VilCache for ShmCacheBackend {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.cache.get(key)
    }

    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) {
        if let Some(d) = ttl {
            self.cache.put_with_ttl(key, value, d);
        } else {
            self.cache.put(key, value);
        }
    }

    async fn del(&self, key: &str) -> bool {
        self.cache.invalidate(key);
        true
    }

    async fn exists(&self, key: &str) -> bool {
        self.cache.get(key).is_some()
    }
}
