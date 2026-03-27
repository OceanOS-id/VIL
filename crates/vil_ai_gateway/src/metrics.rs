use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Serialize, Deserialize};

/// Gateway-level metrics.
pub struct GatewayMetrics {
    pub total_requests: AtomicU64,
    pub total_successes: AtomicU64,
    pub total_failures: AtomicU64,
    pub total_failovers: AtomicU64,
    pub total_circuit_rejections: AtomicU64,
}

/// Serializable snapshot of gateway metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub total_requests: u64,
    pub total_successes: u64,
    pub total_failures: u64,
    pub total_failovers: u64,
    pub total_circuit_rejections: u64,
    pub success_rate: f64,
}

impl GatewayMetrics {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            total_successes: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
            total_failovers: AtomicU64::new(0),
            total_circuit_rejections: AtomicU64::new(0),
        }
    }

    pub fn record_request(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_success(&self) {
        self.total_successes.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        self.total_failures.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_failover(&self) {
        self.total_failovers.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_circuit_rejection(&self) {
        self.total_circuit_rejections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        let requests = self.total_requests.load(Ordering::Relaxed);
        let successes = self.total_successes.load(Ordering::Relaxed);
        let failures = self.total_failures.load(Ordering::Relaxed);
        let failovers = self.total_failovers.load(Ordering::Relaxed);
        let circuit_rejections = self.total_circuit_rejections.load(Ordering::Relaxed);
        let success_rate = if requests > 0 {
            successes as f64 / requests as f64
        } else {
            0.0
        };
        MetricsSnapshot {
            total_requests: requests,
            total_successes: successes,
            total_failures: failures,
            total_failovers: failovers,
            total_circuit_rejections: circuit_rejections,
            success_rate,
        }
    }
}

impl Default for GatewayMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_snapshot() {
        let m = GatewayMetrics::new();
        m.record_request();
        m.record_request();
        m.record_success();
        m.record_failure();
        m.record_failover();
        m.record_circuit_rejection();

        let snap = m.snapshot();
        assert_eq!(snap.total_requests, 2);
        assert_eq!(snap.total_successes, 1);
        assert_eq!(snap.total_failures, 1);
        assert_eq!(snap.total_failovers, 1);
        assert_eq!(snap.total_circuit_rejections, 1);
        assert!((snap.success_rate - 0.5).abs() < 0.01);
    }
}
