// =============================================================================
// VIL Database Plugin — Redis
// =============================================================================
//
// Redis integration for vil-server:
//   - Connection pooling via redis::aio::ConnectionManager
//   - Cache helpers (get/set/del with TTL)
//   - Pub/Sub bridge to vil-server EventBus
//   - Session store backend
//
// This crate uses real Redis connections via the `redis` crate
// with ConnectionManager for automatic reconnection.

pub mod pool;
pub mod cache;
pub mod config;

pub use pool::RedisPool;
pub use cache::RedisCache;
pub use config::RedisConfig;
