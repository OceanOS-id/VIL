use std::time::Duration;
use crate::cache_trait::VilCache;

/// Redis-backed cache using vil_db_redis.
pub struct RedisCacheBackend {
    cache: vil_db_redis::RedisCache,
}

impl RedisCacheBackend {
    pub fn new(cache: vil_db_redis::RedisCache) -> Self {
        Self { cache }
    }
}

#[async_trait::async_trait]
impl VilCache for RedisCacheBackend {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.cache.get(key).await.map(|s| s.into_bytes())
    }

    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) {
        let s = String::from_utf8_lossy(value).to_string();
        self.cache.set(key, &s, ttl).await;
    }

    async fn del(&self, key: &str) -> bool {
        self.cache.del(key).await
    }

    async fn exists(&self, key: &str) -> bool {
        self.cache.get(key).await.is_some()
    }
}
