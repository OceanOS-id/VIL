// =============================================================================
// VIL DB sea-orm — Connection Pool (impl DbPool)
// =============================================================================

use async_trait::async_trait;
use sea_orm::{Database, DatabaseConnection, ConnectOptions, DbErr};
use std::sync::Arc;
use std::time::Duration;

use crate::config::SeaOrmConfig;
use crate::metrics::OrmMetrics;

/// sea-orm connection pool implementing vil_server_db::DbPool.
pub struct SeaOrmPool {
    conn: DatabaseConnection,
    config: SeaOrmConfig,
    metrics: Arc<OrmMetrics>,
    pool_name: String,
}

impl SeaOrmPool {
    /// Connect using config.
    pub async fn connect(name: &str, config: SeaOrmConfig) -> Result<Self, DbErr> {
        let mut opt = ConnectOptions::new(&config.url);
        opt.max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
            .idle_timeout(Duration::from_secs(config.idle_timeout_secs));

        if let Some(schema) = &config.schema {
            opt.set_schema_search_path(schema);
        }

        let conn = Database::connect(opt).await?;

        Ok(Self {
            conn, config,
            metrics: Arc::new(OrmMetrics::new()),
            pool_name: name.to_string(),
        })
    }

    /// Get the underlying DatabaseConnection.
    pub fn conn(&self) -> &DatabaseConnection { &self.conn }

    pub fn name(&self) -> &str { &self.pool_name }
    pub fn config(&self) -> &SeaOrmConfig { &self.config }
    pub fn metrics(&self) -> &Arc<OrmMetrics> { &self.metrics }

    /// Execute raw SQL.
    pub async fn execute_raw(&self, sql: &str) -> Result<u64, DbErr> {
        use sea_orm::{ConnectionTrait, Statement};
        let start = std::time::Instant::now();
        let result = self.conn.execute(Statement::from_string(
            self.conn.get_database_backend(), sql.to_string()
        )).await?;
        self.metrics.record_query(start.elapsed().as_micros() as u64, false);
        Ok(result.rows_affected())
    }

    pub async fn close(&self) {
        self.conn.clone().close().await.ok();
    }
}

#[async_trait]
impl vil_server_db::DbPool for SeaOrmPool {
    type Connection = DatabaseConnection;
    type Error = DbErr;

    async fn acquire(&self) -> Result<Self::Connection, Self::Error> {
        self.metrics.record_acquire();
        Ok(self.conn.clone())
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        self.conn.ping().await?;
        self.metrics.record_health_check(true);
        Ok(())
    }

    async fn close(&self) {
        self.conn.clone().close().await.ok();
    }
}

impl Clone for SeaOrmPool {
    fn clone(&self) -> Self {
        Self {
            conn: self.conn.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
            pool_name: self.pool_name.clone(),
        }
    }
}
