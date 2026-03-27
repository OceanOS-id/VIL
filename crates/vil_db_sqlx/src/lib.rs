// =============================================================================
// VIL Database Plugin — sqlx (PostgreSQL, MySQL, SQLite)
// =============================================================================
//
// Implements the DbPool trait from vil_server_db using sqlx.
// Features:
//   - Multi-pool support (per-service database pools)
//   - Connection metrics (active, idle, query count, latency)
//   - Health check integration
//   - Config hot-reload (change pool size without restart)
//   - Plugin manifest for Admin GUI registration

pub mod pool;
pub mod multi_pool;
pub mod metrics;
pub mod health;
pub mod config;
pub mod extractor;

pub use pool::SqlxPool;
pub use multi_pool::MultiPoolManager;
pub use config::SqlxConfig;
pub use extractor::DbConn;
