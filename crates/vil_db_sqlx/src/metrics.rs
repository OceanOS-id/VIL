// =============================================================================
// VIL DB sqlx — Connection & Query Metrics
// =============================================================================

use std::sync::atomic::{AtomicU64, Ordering};

/// Per-pool metrics tracked atomically.
pub struct PoolMetrics {
    pub queries_total: AtomicU64,
    pub query_errors: AtomicU64,
    pub query_duration_sum_us: AtomicU64,
    pub acquires_total: AtomicU64,
    pub acquire_duration_sum_us: AtomicU64,
    pub health_checks_ok: AtomicU64,
    pub health_checks_fail: AtomicU64,
}

impl PoolMetrics {
    pub fn new() -> Self {
        Self {
            queries_total: AtomicU64::new(0),
            query_errors: AtomicU64::new(0),
            query_duration_sum_us: AtomicU64::new(0),
            acquires_total: AtomicU64::new(0),
            acquire_duration_sum_us: AtomicU64::new(0),
            health_checks_ok: AtomicU64::new(0),
            health_checks_fail: AtomicU64::new(0),
        }
    }

    pub fn record_query(&self, duration_us: u64, is_error: bool) {
        self.queries_total.fetch_add(1, Ordering::Relaxed);
        self.query_duration_sum_us.fetch_add(duration_us, Ordering::Relaxed);
        if is_error {
            self.query_errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_acquire(&self, duration_us: u64) {
        self.acquires_total.fetch_add(1, Ordering::Relaxed);
        self.acquire_duration_sum_us.fetch_add(duration_us, Ordering::Relaxed);
    }

    pub fn record_health_check(&self, ok: bool) {
        if ok {
            self.health_checks_ok.fetch_add(1, Ordering::Relaxed);
        } else {
            self.health_checks_fail.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Export metrics as a snapshot.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let queries = self.queries_total.load(Ordering::Relaxed);
        let dur_sum = self.query_duration_sum_us.load(Ordering::Relaxed);
        MetricsSnapshot {
            queries_total: queries,
            query_errors: self.query_errors.load(Ordering::Relaxed),
            avg_query_us: if queries > 0 { dur_sum / queries } else { 0 },
            acquires_total: self.acquires_total.load(Ordering::Relaxed),
            health_checks_ok: self.health_checks_ok.load(Ordering::Relaxed),
            health_checks_fail: self.health_checks_fail.load(Ordering::Relaxed),
        }
    }

    /// Export as Prometheus text format.
    pub fn to_prometheus(&self, pool_name: &str) -> String {
        let s = self.snapshot();
        format!(
            "vil_db_queries_total{{pool=\"{}\"}} {}\n\
             vil_db_query_errors{{pool=\"{}\"}} {}\n\
             vil_db_query_avg_us{{pool=\"{}\"}} {}\n\
             vil_db_acquires_total{{pool=\"{}\"}} {}\n\
             vil_db_health_ok{{pool=\"{}\"}} {}\n\
             vil_db_health_fail{{pool=\"{}\"}} {}\n",
            pool_name, s.queries_total,
            pool_name, s.query_errors,
            pool_name, s.avg_query_us,
            pool_name, s.acquires_total,
            pool_name, s.health_checks_ok,
            pool_name, s.health_checks_fail,
        )
    }
}

impl Default for PoolMetrics {
    fn default() -> Self { Self::new() }
}

/// Metrics snapshot (serializable).
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricsSnapshot {
    pub queries_total: u64,
    pub query_errors: u64,
    pub avg_query_us: u64,
    pub acquires_total: u64,
    pub health_checks_ok: u64,
    pub health_checks_fail: u64,
}
