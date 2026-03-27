// =============================================================================
// VIL DB sqlx — DbConn Extractor for Axum Handlers
// =============================================================================
//
// Provides database connection injection into handler functions.
//
// Usage:
//   async fn list_users(db: DbConn) -> Json<Vec<User>> {
//       let users = sqlx::query_as("SELECT * FROM users")
//           .fetch_all(db.pool())
//           .await?;
//       Json(users)
//   }

use std::sync::Arc;

use crate::pool::SqlxPool;

/// Database connection handle for use in Axum handlers.
///
/// Provides access to the underlying sqlx AnyPool for executing queries.
/// The pool is reference-counted and cheap to clone.
#[derive(Clone)]
pub struct DbConn {
    pool: Arc<SqlxPool>,
    pool_name: String,
}

impl DbConn {
    pub fn new(pool: Arc<SqlxPool>, pool_name: &str) -> Self {
        Self {
            pool,
            pool_name: pool_name.to_string(),
        }
    }

    /// Get the underlying sqlx AnyPool for queries.
    pub fn pool(&self) -> &sqlx::pool::Pool<sqlx::Any> {
        self.pool.inner()
    }

    /// Get the pool name.
    pub fn pool_name(&self) -> &str {
        &self.pool_name
    }

    /// Execute a raw SQL query.
    pub async fn execute(&self, sql: &str) -> Result<u64, sqlx::Error> {
        self.pool.execute_raw(sql).await
    }

    /// Get pool metrics snapshot.
    pub fn metrics(&self) -> crate::metrics::MetricsSnapshot {
        self.pool.metrics().snapshot()
    }

    /// Get pool size info.
    pub fn size_info(&self) -> crate::pool::PoolSizeInfo {
        self.pool.size_info()
    }
}

impl std::fmt::Debug for DbConn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbConn")
            .field("pool_name", &self.pool_name)
            .finish()
    }
}
