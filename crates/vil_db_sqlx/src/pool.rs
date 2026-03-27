// =============================================================================
// VIL DB sqlx — Connection Pool (impl DbPool)
// =============================================================================

use async_trait::async_trait;
use sqlx::pool::{Pool, PoolOptions};
use sqlx::Any;

type AnyPool = Pool<Any>;
type AnyPoolOptions = PoolOptions<Any>;
use std::sync::Arc;
use std::time::Duration;

use crate::config::SqlxConfig;
use crate::metrics::PoolMetrics;

/// sqlx connection pool implementing vil_server_db::DbPool.
pub struct SqlxPool {
    pool: AnyPool,
    config: SqlxConfig,
    metrics: Arc<PoolMetrics>,
    pool_name: String,
}

impl SqlxPool {
    /// Connect to a database using the provided config.
    pub async fn connect(name: &str, config: SqlxConfig) -> Result<Self, sqlx::Error> {
        // Install default drivers
        #[cfg(feature = "sqlite")]
        sqlx::any::install_default_drivers();

        let pool = AnyPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.connect_timeout_secs))
            .idle_timeout(Some(Duration::from_secs(config.idle_timeout_secs)))
            .connect(&config.url)
            .await?;

        tracing::info!(
            pool = %name,
            driver = %config.driver,
            max_conn = config.max_connections,
            "sqlx pool connected"
        );

        Ok(Self {
            pool,
            config,
            metrics: Arc::new(PoolMetrics::new()),
            pool_name: name.to_string(),
        })
    }

    /// Get the underlying sqlx AnyPool (for raw queries).
    pub fn inner(&self) -> &AnyPool {
        &self.pool
    }

    /// Get pool name.
    pub fn name(&self) -> &str {
        &self.pool_name
    }

    /// Get pool configuration.
    pub fn config(&self) -> &SqlxConfig {
        &self.config
    }

    /// Get pool metrics.
    pub fn metrics(&self) -> &Arc<PoolMetrics> {
        &self.metrics
    }

    /// Get pool size info.
    pub fn size_info(&self) -> PoolSizeInfo {
        PoolSizeInfo {
            max: self.config.max_connections,
            min: self.config.min_connections,
            current: self.pool.size(),
            idle: self.pool.num_idle() as u32,
        }
    }

    /// Execute a raw SQL query (for health check, migrations, etc).
    pub async fn execute_raw(&self, sql: &str) -> Result<u64, sqlx::Error> {
        let start = std::time::Instant::now();
        let result = sqlx::query(sql).execute(&self.pool).await?;
        let duration_us = start.elapsed().as_micros() as u64;
        self.metrics.record_query(duration_us, false);
        Ok(result.rows_affected())
    }

    /// Close the pool gracefully.
    pub async fn close(&self) {
        self.pool.close().await;
        tracing::info!(pool = %self.pool_name, "sqlx pool closed");
    }
}

#[async_trait]
impl vil_server_db::DbPool for SqlxPool {
    type Connection = sqlx::pool::PoolConnection<sqlx::Any>;
    type Error = sqlx::Error;

    async fn acquire(&self) -> Result<Self::Connection, Self::Error> {
        let start = std::time::Instant::now();
        let conn = self.pool.acquire().await?;
        let duration_us = start.elapsed().as_micros() as u64;
        self.metrics.record_acquire(duration_us);
        Ok(conn)
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        self.metrics.record_health_check(true);
        Ok(())
    }

    async fn close(&self) {
        self.pool.close().await;
    }
}

/// Pool size information.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PoolSizeInfo {
    pub max: u32,
    pub min: u32,
    pub current: u32,
    pub idle: u32,
}

impl Clone for SqlxPool {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
            pool_name: self.pool_name.clone(),
        }
    }
}
