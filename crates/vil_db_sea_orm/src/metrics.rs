// =============================================================================
// VIL DB sea-orm — ORM Metrics
// =============================================================================

use std::sync::atomic::{AtomicU64, Ordering};

pub struct OrmMetrics {
    pub queries_total: AtomicU64,
    pub query_errors: AtomicU64,
    pub query_duration_sum_ns: AtomicU64,
    pub acquires_total: AtomicU64,
    pub health_ok: AtomicU64,
    pub health_fail: AtomicU64,
    pub migrations_run: AtomicU64,
}

impl OrmMetrics {
    pub fn new() -> Self {
        Self {
            queries_total: AtomicU64::new(0),
            query_errors: AtomicU64::new(0),
            query_duration_sum_ns: AtomicU64::new(0),
            acquires_total: AtomicU64::new(0),
            health_ok: AtomicU64::new(0),
            health_fail: AtomicU64::new(0),
            migrations_run: AtomicU64::new(0),
        }
    }

    pub fn record_query(&self, duration_ns: u64, is_error: bool) {
        self.queries_total.fetch_add(1, Ordering::Relaxed);
        self.query_duration_sum_ns
            .fetch_add(duration_ns, Ordering::Relaxed);
        if is_error {
            self.query_errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_acquire(&self) {
        self.acquires_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_health_check(&self, ok: bool) {
        if ok {
            self.health_ok.fetch_add(1, Ordering::Relaxed);
        } else {
            self.health_fail.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_migration(&self) {
        self.migrations_run.fetch_add(1, Ordering::Relaxed);
    }

    pub fn to_prometheus(&self, pool_name: &str) -> String {
        format!(
            "vil_orm_queries_total{{pool=\"{}\"}} {}\n\
             vil_orm_query_errors{{pool=\"{}\"}} {}\n\
             vil_orm_acquires{{pool=\"{}\"}} {}\n\
             vil_orm_migrations{{pool=\"{}\"}} {}\n",
            pool_name,
            self.queries_total.load(Ordering::Relaxed),
            pool_name,
            self.query_errors.load(Ordering::Relaxed),
            pool_name,
            self.acquires_total.load(Ordering::Relaxed),
            pool_name,
            self.migrations_run.load(Ordering::Relaxed),
        )
    }
}

impl Default for OrmMetrics {
    fn default() -> Self {
        Self::new()
    }
}
