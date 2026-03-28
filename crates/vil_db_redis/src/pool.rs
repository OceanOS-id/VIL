// =============================================================================
// VIL DB Redis — Connection Pool (real redis ConnectionManager)
// =============================================================================

use async_trait::async_trait;
use redis::aio::ConnectionManager;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::config::RedisConfig;

/// Redis error type.
#[derive(Debug)]
pub struct RedisError(pub String);

impl std::fmt::Display for RedisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Redis error: {}", self.0)
    }
}

impl std::error::Error for RedisError {}

/// Redis connection pool backed by real redis ConnectionManager.
#[derive(Clone)]
pub struct RedisPool {
    config: RedisConfig,
    conn: ConnectionManager,
    ops_count: std::sync::Arc<AtomicU64>,
    pool_name: String,
}

impl RedisPool {
    pub async fn connect(name: &str, config: RedisConfig) -> Result<Self, String> {
        let client = redis::Client::open(config.url.as_str())
            .map_err(|e| format!("Redis client open failed: {}", e))?;

        let conn = ConnectionManager::new(client).await
            .map_err(|e| format!("Redis ConnectionManager failed: {}", e))?;

        Ok(Self {
            config,
            conn,
            ops_count: std::sync::Arc::new(AtomicU64::new(0)),
            pool_name: name.to_string(),
        })
    }

    pub fn name(&self) -> &str { &self.pool_name }
    pub fn config(&self) -> &RedisConfig { &self.config }
    pub fn ops_count(&self) -> u64 { self.ops_count.load(Ordering::Relaxed) }

    /// Get a value by key (async, real Redis).
    pub async fn get(&self, key: &str) -> Option<String> {
        self.ops_count.fetch_add(1, Ordering::Relaxed);
        let mut conn = self.conn.clone();
        redis::cmd("GET").arg(key).query_async::<Option<String>>(&mut conn).await.ok().flatten()
    }

    /// Set a value (async, real Redis).
    pub async fn set(&self, key: &str, value: &str) {
        self.ops_count.fetch_add(1, Ordering::Relaxed);
        let mut conn = self.conn.clone();
        let _: Result<(), _> = redis::cmd("SET").arg(key).arg(value)
            .query_async(&mut conn).await;
    }

    /// Set a value with TTL in seconds (async, real Redis).
    pub async fn set_ex(&self, key: &str, value: &str, ttl_secs: u64) {
        self.ops_count.fetch_add(1, Ordering::Relaxed);
        let mut conn = self.conn.clone();
        let _: Result<(), _> = redis::cmd("SET").arg(key).arg(value).arg("EX").arg(ttl_secs)
            .query_async(&mut conn).await;
    }

    /// Delete a key (async, real Redis).
    pub async fn del(&self, key: &str) -> bool {
        self.ops_count.fetch_add(1, Ordering::Relaxed);
        let mut conn = self.conn.clone();
        redis::cmd("DEL").arg(key).query_async::<i64>(&mut conn).await.unwrap_or(0) > 0
    }

    /// Get number of keys matching a pattern (uses DBSIZE for all keys).
    pub async fn keys_count(&self) -> usize {
        let mut conn = self.conn.clone();
        redis::cmd("DBSIZE").query_async::<usize>(&mut conn).await.unwrap_or(0)
    }

    /// Ping the Redis server.
    pub async fn ping(&self) -> Result<String, String> {
        let mut conn = self.conn.clone();
        redis::cmd("PING").query_async::<String>(&mut conn).await
            .map_err(|e| format!("Redis PING failed: {}", e))
    }

    pub async fn close(&self) {
    }

    /// Access the underlying ConnectionManager for advanced use cases.
    pub fn inner(&self) -> ConnectionManager { self.conn.clone() }
}

#[async_trait]
impl vil_server_db::DbPool for RedisPool {
    type Connection = ConnectionManager;
    type Error = RedisError;

    async fn acquire(&self) -> Result<Self::Connection, Self::Error> {
        Ok(self.conn.clone())
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        self.ping().await.map(|_| ()).map_err(|e| RedisError(e))
    }

    async fn close(&self) { /* ConnectionManager handles cleanup */ }
}
