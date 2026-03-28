// =============================================================================
// Sidecar Metrics — Per-sidecar atomic counters + Prometheus export
// =============================================================================

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Per-sidecar metrics (lock-free atomic counters).
pub struct SidecarMetrics {
    pub invocations: AtomicU64,
    pub errors: AtomicU64,
    pub timeouts: AtomicU64,
    pub in_flight: AtomicU64,
    pub total_latency_us: AtomicU64,
    pub health_failures: AtomicU64,
    started_at: Instant,
}

impl SidecarMetrics {
    pub fn new() -> Self {
        Self {
            invocations: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            timeouts: AtomicU64::new(0),
            in_flight: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            health_failures: AtomicU64::new(0),
            started_at: Instant::now(),
        }
    }

    /// Record the start of an invocation.
    pub fn invoke_start(&self) {
        self.invocations.fetch_add(1, Ordering::Relaxed);
        self.in_flight.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a successful invocation with latency.
    pub fn invoke_ok(&self, latency_us: u64) {
        self.in_flight.fetch_sub(1, Ordering::Relaxed);
        self.total_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
    }

    /// Record a failed invocation.
    pub fn invoke_error(&self) {
        self.in_flight.fetch_sub(1, Ordering::Relaxed);
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a timeout.
    pub fn invoke_timeout(&self) {
        self.in_flight.fetch_sub(1, Ordering::Relaxed);
        self.timeouts.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a health check failure.
    pub fn health_failure(&self) {
        self.health_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// Reset health failure counter (on successful health check).
    pub fn health_ok(&self) {
        self.health_failures.store(0, Ordering::Relaxed);
    }

    /// Snapshot of all metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let invocations = self.invocations.load(Ordering::Relaxed);
        let total_latency = self.total_latency_us.load(Ordering::Relaxed);
        let successful = invocations.saturating_sub(
            self.errors.load(Ordering::Relaxed) + self.timeouts.load(Ordering::Relaxed),
        );
        MetricsSnapshot {
            invocations,
            errors: self.errors.load(Ordering::Relaxed),
            timeouts: self.timeouts.load(Ordering::Relaxed),
            in_flight: self.in_flight.load(Ordering::Relaxed),
            avg_latency_us: if successful > 0 {
                total_latency / successful
            } else {
                0
            },
            health_failures: self.health_failures.load(Ordering::Relaxed),
            uptime_secs: self.started_at.elapsed().as_secs(),
        }
    }

    /// Export as Prometheus text format lines.
    pub fn to_prometheus(&self, name: &str) -> String {
        let s = self.snapshot();
        format!(
            "vil_sidecar_invocations_total{{sidecar=\"{}\"}} {}\n\
             vil_sidecar_errors_total{{sidecar=\"{}\"}} {}\n\
             vil_sidecar_timeouts_total{{sidecar=\"{}\"}} {}\n\
             vil_sidecar_in_flight{{sidecar=\"{}\"}} {}\n\
             vil_sidecar_avg_latency_us{{sidecar=\"{}\"}} {}\n\
             vil_sidecar_health_failures{{sidecar=\"{}\"}} {}\n",
            name,
            s.invocations,
            name,
            s.errors,
            name,
            s.timeouts,
            name,
            s.in_flight,
            name,
            s.avg_latency_us,
            name,
            s.health_failures,
        )
    }
}

impl Default for SidecarMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Immutable snapshot of metrics at a point in time.
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub invocations: u64,
    pub errors: u64,
    pub timeouts: u64,
    pub in_flight: u64,
    pub avg_latency_us: u64,
    pub health_failures: u64,
    pub uptime_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_lifecycle() {
        let m = SidecarMetrics::new();

        m.invoke_start();
        m.invoke_start();
        assert_eq!(m.in_flight.load(Ordering::Relaxed), 2);

        m.invoke_ok(100);
        assert_eq!(m.in_flight.load(Ordering::Relaxed), 1);

        m.invoke_error();
        assert_eq!(m.in_flight.load(Ordering::Relaxed), 0);

        let s = m.snapshot();
        assert_eq!(s.invocations, 2);
        assert_eq!(s.errors, 1);
        assert_eq!(s.avg_latency_us, 100);
    }

    #[test]
    fn test_prometheus_format() {
        let m = SidecarMetrics::new();
        m.invoke_start();
        m.invoke_ok(50);
        let prom = m.to_prometheus("fraud-checker");
        assert!(prom.contains("vil_sidecar_invocations_total{sidecar=\"fraud-checker\"} 1"));
        assert!(prom.contains("vil_sidecar_errors_total{sidecar=\"fraud-checker\"} 0"));
    }

    #[test]
    fn test_health_tracking() {
        let m = SidecarMetrics::new();
        m.health_failure();
        m.health_failure();
        assert_eq!(m.health_failures.load(Ordering::Relaxed), 2);
        m.health_ok();
        assert_eq!(m.health_failures.load(Ordering::Relaxed), 0);
    }
}
