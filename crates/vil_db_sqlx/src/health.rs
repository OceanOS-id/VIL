// =============================================================================
// VIL DB sqlx — Health Check
// =============================================================================

use crate::pool::SqlxPool;

/// Health check result for a database pool.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthResult {
    pub pool_name: String,
    pub healthy: bool,
    pub latency_ns: u64,
    pub error: Option<String>,
    pub pool_size: u32,
    pub idle: u32,
}

/// Run a health check on a pool.
pub async fn check_health(pool: &SqlxPool) -> HealthResult {
    let start = std::time::Instant::now();

    match pool.execute_raw("SELECT 1").await {
        Ok(_) => {
            let info = pool.size_info();
            HealthResult {
                pool_name: pool.name().to_string(),
                healthy: true,
                latency_ns: start.elapsed().as_nanos() as u64,
                error: None,
                pool_size: info.current,
                idle: info.idle,
            }
        }
        Err(e) => {
            pool.metrics().record_health_check(false);
            HealthResult {
                pool_name: pool.name().to_string(),
                healthy: false,
                latency_ns: start.elapsed().as_nanos() as u64,
                error: Some(e.to_string()),
                pool_size: 0,
                idle: 0,
            }
        }
    }
}
