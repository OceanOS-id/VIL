// =============================================================================
// VIL DB Redis — Cache Helpers (real Redis backend)
// =============================================================================

use redis::aio::ConnectionManager;
use std::time::Duration;

/// Redis cache with TTL support (real Redis backend).
#[derive(Clone)]
pub struct RedisCache {
    conn: ConnectionManager,
}

impl RedisCache {
    /// Create a cache from a ConnectionManager (use RedisPool::inner() to get one).
    pub fn new(conn: ConnectionManager) -> Self {
        Self { conn }
    }

    /// Create a cache by connecting directly to a Redis URL.
    pub async fn connect(url: &str) -> Result<Self, String> {
        let client = redis::Client::open(url)
            .map_err(|e| format!("Redis client open failed: {}", e))?;
        let conn = ConnectionManager::new(client).await
            .map_err(|e| format!("Redis ConnectionManager failed: {}", e))?;
        Ok(Self { conn })
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let mut conn = self.conn.clone();
        redis::cmd("GET").arg(key).query_async::<Option<String>>(&mut conn).await.ok().flatten()
    }

    pub async fn set(&self, key: &str, value: &str, ttl: Option<Duration>) {
        let mut conn = self.conn.clone();
        if let Some(ttl) = ttl {
            let _: Result<(), _> = redis::cmd("SET")
                .arg(key).arg(value).arg("EX").arg(ttl.as_secs())
                .query_async(&mut conn).await;
        } else {
            let _: Result<(), _> = redis::cmd("SET")
                .arg(key).arg(value)
                .query_async(&mut conn).await;
        }
    }

    pub async fn del(&self, key: &str) -> bool {
        let mut conn = self.conn.clone();
        redis::cmd("DEL").arg(key).query_async::<i64>(&mut conn).await.unwrap_or(0) > 0
    }

    pub async fn set_json<T: serde::Serialize>(&self, key: &str, value: &T, ttl: Option<Duration>) {
        if let Ok(json) = serde_json::to_string(value) {
            self.set(key, &json, ttl).await;
        }
    }

    pub async fn get_json<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let val = self.get(key).await?;
        serde_json::from_str(&val).ok()
    }

    /// Get number of keys in the current database.
    pub async fn keys_count(&self) -> usize {
        let mut conn = self.conn.clone();
        redis::cmd("DBSIZE").query_async::<usize>(&mut conn).await.unwrap_or(0)
    }

    /// Cleanup expired keys is handled automatically by Redis server.
    /// This method is kept for API compatibility but is a no-op.
    pub async fn cleanup_expired(&self) -> usize {
        // Redis handles TTL expiry automatically; no manual cleanup needed.
        0
    }
}
