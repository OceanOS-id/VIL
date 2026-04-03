use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Per-endpoint metrics with atomic counters.
#[derive(Debug)]
pub struct EndpointMetrics {
    pub path: String,
    pub method: String,
    pub requests: AtomicU64,
    pub errors: AtomicU64,
    pub total_latency_ns: AtomicU64,
    pub min_latency_ns: AtomicU64,
    pub max_latency_ns: AtomicU64,
    pub p95_ns: AtomicU64,
    pub p99_ns: AtomicU64,
    pub p999_ns: AtomicU64,
}

impl EndpointMetrics {
    pub fn new(method: &str, path: &str) -> Self {
        Self {
            path: path.to_string(),
            method: method.to_string(),
            requests: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            total_latency_ns: AtomicU64::new(0),
            min_latency_ns: AtomicU64::new(u64::MAX),
            max_latency_ns: AtomicU64::new(0),
            p95_ns: AtomicU64::new(0),
            p99_ns: AtomicU64::new(0),
            p999_ns: AtomicU64::new(0),
        }
    }

    pub fn record(&self, latency_ns: u64, is_error: bool) {
        self.requests.fetch_add(1, Ordering::Relaxed);
        if is_error {
            self.errors.fetch_add(1, Ordering::Relaxed);
        }
        self.total_latency_ns
            .fetch_add(latency_ns, Ordering::Relaxed);
        // Update min (atomic min via compare_exchange loop)
        let mut current = self.min_latency_ns.load(Ordering::Relaxed);
        while latency_ns < current {
            match self.min_latency_ns.compare_exchange_weak(
                current,
                latency_ns,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(c) => current = c,
            }
        }
        // Update max
        let mut current = self.max_latency_ns.load(Ordering::Relaxed);
        while latency_ns > current {
            match self.max_latency_ns.compare_exchange_weak(
                current,
                latency_ns,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(c) => current = c,
            }
        }
    }

    pub fn snapshot(&self) -> EndpointSnapshot {
        let requests = self.requests.load(Ordering::Relaxed);
        let errors = self.errors.load(Ordering::Relaxed);
        let total = self.total_latency_ns.load(Ordering::Relaxed);
        let min = self.min_latency_ns.load(Ordering::Relaxed);
        let max = self.max_latency_ns.load(Ordering::Relaxed);

        EndpointSnapshot {
            path: self.path.clone(),
            method: self.method.clone(),
            requests,
            errors,
            error_rate: if requests > 0 {
                errors as f64 / requests as f64
            } else {
                0.0
            },
            avg_latency_ns: if requests > 0 { total / requests } else { 0 },
            min_latency_ns: if min == u64::MAX { 0 } else { min },
            max_latency_ns: max,
            p95_ns: self.p95_ns.load(Ordering::Relaxed),
            p99_ns: self.p99_ns.load(Ordering::Relaxed),
            p999_ns: self.p999_ns.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EndpointSnapshot {
    pub path: String,
    pub method: String,
    pub requests: u64,
    pub errors: u64,
    pub error_rate: f64,
    pub avg_latency_ns: u64,
    pub min_latency_ns: u64,
    pub max_latency_ns: u64,
    pub p95_ns: u64,
    pub p99_ns: u64,
    pub p999_ns: u64,
}

/// Global metrics collector for all endpoints.
#[derive(Debug)]
pub struct MetricsCollector {
    endpoints: std::sync::Mutex<Vec<Arc<EndpointMetrics>>>,
    started_at: std::sync::Mutex<Option<std::time::Instant>>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            endpoints: std::sync::Mutex::new(Vec::new()),
            started_at: std::sync::Mutex::new(None),
        }
    }

    /// Initialize uptime clock (for sidecar mode where no endpoints are registered).
    pub fn init_uptime(&self) {
        let mut started = self.started_at.lock().unwrap();
        if started.is_none() {
            *started = Some(std::time::Instant::now());
        }
    }

    pub fn register_endpoint(&self, method: &str, path: &str) -> Arc<EndpointMetrics> {
        let mut started = self.started_at.lock().unwrap();
        if started.is_none() {
            *started = Some(std::time::Instant::now());
        }
        drop(started);
        let metrics = Arc::new(EndpointMetrics::new(method, path));
        self.endpoints.lock().unwrap().push(metrics.clone());
        metrics
    }

    pub fn all_snapshots(&self) -> Vec<EndpointSnapshot> {
        self.endpoints
            .lock()
            .unwrap()
            .iter()
            .map(|m| m.snapshot())
            .collect()
    }

    pub fn uptime_secs(&self) -> u64 {
        self.started_at
            .lock()
            .unwrap()
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0)
    }

    pub fn total_requests(&self) -> u64 {
        self.endpoints
            .lock()
            .unwrap()
            .iter()
            .map(|m| m.requests.load(Ordering::Relaxed))
            .sum()
    }

    /// Sync endpoint data from an external metrics source (e.g. HandlerMetricsRegistry).
    /// Creates the endpoint entry if it doesn't exist, then overwrites counters.
    pub fn sync_endpoint(
        &self,
        method: &str,
        path: &str,
        requests: u64,
        errors: u64,
        avg_latency_ns: u64,
        p95_ns: u64,
        p99_ns: u64,
        p999_ns: u64,
    ) {
        let mut started = self.started_at.lock().unwrap();
        if started.is_none() {
            *started = Some(std::time::Instant::now());
        }
        drop(started);

        let mut endpoints = self.endpoints.lock().unwrap();
        let existing = endpoints
            .iter()
            .find(|m| m.method == method && m.path == path);
        if let Some(m) = existing {
            m.requests.store(requests, Ordering::Relaxed);
            m.errors.store(errors, Ordering::Relaxed);
            m.total_latency_ns
                .store(avg_latency_ns * requests, Ordering::Relaxed);
            m.p95_ns.store(p95_ns, Ordering::Relaxed);
            m.p99_ns.store(p99_ns, Ordering::Relaxed);
            m.p999_ns.store(p999_ns, Ordering::Relaxed);
            if requests > 0 && avg_latency_ns > 0 {
                let min = m.min_latency_ns.load(Ordering::Relaxed);
                if avg_latency_ns < min {
                    m.min_latency_ns.store(avg_latency_ns, Ordering::Relaxed);
                }
                let max = m.max_latency_ns.load(Ordering::Relaxed);
                if avg_latency_ns > max || max == 0 {
                    m.max_latency_ns.store(avg_latency_ns, Ordering::Relaxed);
                }
            }
        } else {
            let m = Arc::new(EndpointMetrics::new(method, path));
            m.requests.store(requests, Ordering::Relaxed);
            m.errors.store(errors, Ordering::Relaxed);
            m.total_latency_ns
                .store(avg_latency_ns * requests, Ordering::Relaxed);
            m.p95_ns.store(p95_ns, Ordering::Relaxed);
            m.p99_ns.store(p99_ns, Ordering::Relaxed);
            m.p999_ns.store(p999_ns, Ordering::Relaxed);
            if requests > 0 && avg_latency_ns > 0 {
                m.min_latency_ns.store(avg_latency_ns, Ordering::Relaxed);
                m.max_latency_ns.store(avg_latency_ns, Ordering::Relaxed);
            }
            endpoints.push(m);
        }
    }

    /// Sync with actual min/max from HandlerMetricsRegistry.
    pub fn sync_endpoint_full(
        &self,
        method: &str,
        path: &str,
        requests: u64,
        errors: u64,
        avg_latency_ns: u64,
        min_ns: u64,
        max_ns: u64,
        p95_ns: u64,
        p99_ns: u64,
        p999_ns: u64,
    ) {
        let mut started = self.started_at.lock().unwrap();
        if started.is_none() {
            *started = Some(std::time::Instant::now());
        }
        drop(started);

        let mut endpoints = self.endpoints.lock().unwrap();
        let existing = endpoints
            .iter()
            .find(|m| m.method == method && m.path == path);
        if let Some(m) = existing {
            m.requests.store(requests, Ordering::Relaxed);
            m.errors.store(errors, Ordering::Relaxed);
            m.total_latency_ns
                .store(avg_latency_ns * requests, Ordering::Relaxed);
            m.min_latency_ns.store(
                if min_ns == u64::MAX { 0 } else { min_ns },
                Ordering::Relaxed,
            );
            m.max_latency_ns.store(max_ns, Ordering::Relaxed);
            m.p95_ns.store(p95_ns, Ordering::Relaxed);
            m.p99_ns.store(p99_ns, Ordering::Relaxed);
            m.p999_ns.store(p999_ns, Ordering::Relaxed);
        } else {
            let m = Arc::new(EndpointMetrics::new(method, path));
            m.requests.store(requests, Ordering::Relaxed);
            m.errors.store(errors, Ordering::Relaxed);
            m.total_latency_ns
                .store(avg_latency_ns * requests, Ordering::Relaxed);
            m.min_latency_ns.store(
                if min_ns == u64::MAX { 0 } else { min_ns },
                Ordering::Relaxed,
            );
            m.max_latency_ns.store(max_ns, Ordering::Relaxed);
            m.p95_ns.store(p95_ns, Ordering::Relaxed);
            m.p99_ns.store(p99_ns, Ordering::Relaxed);
            m.p999_ns.store(p999_ns, Ordering::Relaxed);
            endpoints.push(m);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_metrics() {
        let m = EndpointMetrics::new("GET", "/api/users");
        m.record(100, false);
        m.record(200, false);
        m.record(300, true);

        let snap = m.snapshot();
        assert_eq!(snap.requests, 3);
        assert_eq!(snap.errors, 1);
        assert_eq!(snap.min_latency_ns, 100);
        assert_eq!(snap.max_latency_ns, 300);
        assert_eq!(snap.avg_latency_ns, 200); // (100+200+300)/3
    }

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();
        let m1 = collector.register_endpoint("GET", "/users");
        let m2 = collector.register_endpoint("POST", "/users");

        m1.record(50, false);
        m2.record(100, false);

        let snaps = collector.all_snapshots();
        assert_eq!(snaps.len(), 2);
        assert_eq!(collector.total_requests(), 2);
    }
}
