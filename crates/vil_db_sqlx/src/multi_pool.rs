// =============================================================================
// VIL DB sqlx — Multi-Pool Manager (per-service pools)
// =============================================================================

use dashmap::DashMap;
use std::sync::Arc;

use crate::config::SqlxConfig;
use crate::health::{self, HealthResult};
use crate::pool::SqlxPool;

/// Manages multiple named database pools (one per service or shared).
pub struct MultiPoolManager {
    pools: DashMap<String, Arc<SqlxPool>>,
}

impl MultiPoolManager {
    pub fn new() -> Self {
        Self {
            pools: DashMap::new(),
        }
    }

    /// Create and register a pool.
    pub async fn add_pool(&self, name: &str, config: SqlxConfig) -> Result<(), String> {
        let pool = SqlxPool::connect(name, config)
            .await
            .map_err(|e| format!("Failed to connect pool '{}': {}", name, e))?;

        self.pools.insert(name.to_string(), Arc::new(pool));
        Ok(())
    }

    /// Get a pool by name.
    pub fn get(&self, name: &str) -> Option<Arc<SqlxPool>> {
        self.pools.get(name).map(|p| p.value().clone())
    }

    /// Get a pool that serves a specific service.
    pub fn get_for_service(&self, service: &str) -> Option<Arc<SqlxPool>> {
        for entry in self.pools.iter() {
            if entry.value().config().is_for_service(service) {
                return Some(entry.value().clone());
            }
        }
        None
    }

    /// Remove and close a pool.
    pub async fn remove_pool(&self, name: &str) {
        if let Some((_, pool)) = self.pools.remove(name) {
            pool.close().await;
        }
    }

    /// Health check all pools.
    pub async fn health_check_all(&self) -> Vec<HealthResult> {
        let mut results = Vec::new();
        for entry in self.pools.iter() {
            let result = health::check_health(entry.value()).await;
            results.push(result);
        }
        results
    }

    /// Export all pool metrics as Prometheus text.
    pub fn prometheus_metrics(&self) -> String {
        let mut out = String::new();
        for entry in self.pools.iter() {
            out.push_str(&entry.value().metrics().to_prometheus(entry.key()));
        }
        out
    }

    /// List pool names.
    pub fn pool_names(&self) -> Vec<String> {
        self.pools.iter().map(|e| e.key().clone()).collect()
    }

    /// Pool count.
    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }

    /// Close all pools gracefully.
    pub async fn close_all(&self) {
        for entry in self.pools.iter() {
            entry.value().close().await;
        }
        self.pools.clear();
    }
}

impl Default for MultiPoolManager {
    fn default() -> Self {
        Self::new()
    }
}
