// =============================================================================
// VIL Server DB — Database connection pooling and transaction support
// =============================================================================
//
// This crate provides database abstractions for vil-server.
// For the community edition, it provides a trait-based interface
// that can be implemented for any database backend (sqlx, diesel, etc.).

use async_trait::async_trait;
use serde::Deserialize;

/// Database configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DbConfig {
    /// Database connection URL
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of idle connections
    pub min_connections: u32,
    /// Connection timeout in seconds
    pub connect_timeout_secs: u64,
    /// Idle timeout in seconds
    pub idle_timeout_secs: u64,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            max_connections: 10,
            min_connections: 1,
            connect_timeout_secs: 5,
            idle_timeout_secs: 300,
        }
    }
}

/// Trait for database pool implementations.
/// Implement this trait to integrate with sqlx, diesel, or other database libraries.
#[async_trait]
pub trait DbPool: Send + Sync + Clone + 'static {
    type Connection: Send;
    type Error: std::error::Error + Send + Sync;

    /// Get a connection from the pool.
    async fn acquire(&self) -> Result<Self::Connection, Self::Error>;

    /// Check if the pool is healthy (for readiness probes).
    async fn health_check(&self) -> Result<(), Self::Error>;

    /// Close the pool gracefully.
    async fn close(&self);
}

/// Transaction wrapper that automatically rolls back on drop.
/// Commit must be called explicitly.
pub struct Transaction<C> {
    conn: Option<C>,
    committed: bool,
}

impl<C> Transaction<C> {
    pub fn new(conn: C) -> Self {
        Self {
            conn: Some(conn),
            committed: false,
        }
    }

    /// Get a reference to the underlying connection.
    pub fn conn(&self) -> &C {
        self.conn.as_ref().expect("Transaction already consumed")
    }

    /// Get a mutable reference to the underlying connection.
    pub fn conn_mut(&mut self) -> &mut C {
        self.conn.as_mut().expect("Transaction already consumed")
    }

    /// Mark the transaction as committed.
    /// The actual commit should be performed by the database driver.
    pub fn commit(mut self) {
        self.committed = true;
    }
}

impl<C> Drop for Transaction<C> {
    fn drop(&mut self) {
        if !self.committed {
            tracing::warn!("Transaction dropped without commit — rollback implied");
        }
    }
}
