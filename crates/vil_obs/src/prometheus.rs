// =============================================================================
// VIL Prometheus Metrics Exporter
// =============================================================================

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// VIL metrics for Prometheus
#[derive(Debug, Clone)]
pub struct VilMetrics {
    requests_total: Arc<AtomicU64>,
    requests_in_flight: Arc<AtomicU64>,
    request_duration_ms_sum: Arc<AtomicU64>,
    request_duration_ms_count: Arc<AtomicU64>,
    queue_depth: Arc<AtomicU64>,
    shm_used_bytes: Arc<AtomicU64>,
    route_errors_total: Arc<AtomicU64>,
    upstream_errors_total: Arc<AtomicU64>,
}

impl Default for VilMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl VilMetrics {
    pub fn new() -> Self {
        Self {
            requests_total: Arc::new(AtomicU64::new(0)),
            requests_in_flight: Arc::new(AtomicU64::new(0)),
            request_duration_ms_sum: Arc::new(AtomicU64::new(0)),
            request_duration_ms_count: Arc::new(AtomicU64::new(0)),
            queue_depth: Arc::new(AtomicU64::new(0)),
            shm_used_bytes: Arc::new(AtomicU64::new(0)),
            route_errors_total: Arc::new(AtomicU64::new(0)),
            upstream_errors_total: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn request_start(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        self.requests_in_flight.fetch_add(1, Ordering::Relaxed);
    }

    pub fn request_end(&self, duration_ms: u64) {
        self.requests_in_flight.fetch_sub(1, Ordering::Relaxed);
        self.request_duration_ms_sum
            .fetch_add(duration_ms, Ordering::Relaxed);
        self.request_duration_ms_count
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn upstream_error(&self) {
        self.upstream_errors_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn route_error(&self) {
        self.route_errors_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn set_queue_depth(&self, depth: u64) {
        self.queue_depth.store(depth, Ordering::Relaxed);
    }

    pub fn set_shm_used_bytes(&self, bytes: u64) {
        self.shm_used_bytes.store(bytes, Ordering::Relaxed);
    }

    /// Sync metrics from RuntimeCounters and LatencyTracker snapshots.
    /// Call this periodically to keep Prometheus metrics up to date with the runtime.
    pub fn sync_from_runtime(
        &self,
        counters: &crate::CounterSnapshot,
        latency: &crate::LatencySnapshot,
    ) {
        // Map runtime counters to Prometheus metrics
        self.requests_total.store(counters.publishes, Ordering::Relaxed);
        self.route_errors_total.store(counters.drops + counters.crashes, Ordering::Relaxed);

        // Update latency from tracker
        if latency.count > 0 {
            // Convert nanoseconds to milliseconds
            let mean_ms = latency.mean_ns / 1_000_000;
            self.request_duration_ms_sum.store(mean_ms * latency.count, Ordering::Relaxed);
            self.request_duration_ms_count.store(latency.count, Ordering::Relaxed);
        }
    }

    /// Generate Prometheus text exposition format output.
    pub fn to_prometheus(&self) -> String {
        let requests_total = self.requests_total.load(Ordering::Relaxed);
        let requests_in_flight = self.requests_in_flight.load(Ordering::Relaxed);
        let duration_sum = self.request_duration_ms_sum.load(Ordering::Relaxed);
        let duration_count = self.request_duration_ms_count.load(Ordering::Relaxed);
        let queue_depth = self.queue_depth.load(Ordering::Relaxed);
        let shm_used = self.shm_used_bytes.load(Ordering::Relaxed);
        let route_errors = self.route_errors_total.load(Ordering::Relaxed);
        let upstream_errors = self.upstream_errors_total.load(Ordering::Relaxed);

        let avg_duration = if duration_count > 0 {
            duration_sum as f64 / duration_count as f64
        } else {
            0.0
        };

        format!(
            r#"# HELP vil_requests_total Total number of requests
# TYPE vil_requests_total counter
vil_requests_total {}

# HELP vil_requests_in_flight Number of requests currently being processed
# TYPE vil_requests_in_flight gauge
vil_requests_in_flight {}

# HELP vil_request_duration_ms Average request duration in milliseconds
# TYPE vil_request_duration_ms gauge
vil_request_duration_ms {}

# HELP vil_queue_depth Number of messages in queue
# TYPE vil_queue_depth gauge
vil_queue_depth {}

# HELP vil_shm_used_bytes Shared memory usage in bytes
# TYPE vil_shm_used_bytes gauge
vil_shm_used_bytes {}

# HELP vil_route_errors_total Total number of route errors
# TYPE vil_route_errors_total counter
vil_route_errors_total {}

# HELP vil_upstream_errors_total Total number of upstream errors
# TYPE vil_upstream_errors_total counter
vil_upstream_errors_total {}
"#,
            requests_total,
            requests_in_flight,
            avg_duration,
            queue_depth,
            shm_used,
            route_errors,
            upstream_errors
        )
    }
}
