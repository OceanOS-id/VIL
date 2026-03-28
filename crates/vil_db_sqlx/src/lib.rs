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

pub mod config;
pub mod extractor;
pub mod health;
pub mod metrics;
pub mod multi_pool;
pub mod pool;

pub use config::SqlxConfig;
pub use extractor::DbConn;
pub use multi_pool::MultiPoolManager;
pub use pool::SqlxPool;
