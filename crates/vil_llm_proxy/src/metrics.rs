//! Proxy metrics — requests, tokens, cost, latency tracking.

use std::sync::atomic::{AtomicU64, Ordering};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use vil_macros::VilAiState;

/// Proxy-wide metrics with atomic counters.
pub struct ProxyMetrics {
    pub total_requests: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub rate_limited: AtomicU64,
    pub errors: AtomicU64,
    pub total_tokens: AtomicU64,
    pub total_cost_cents: AtomicU64,
    pub per_model_requests: DashMap<String, AtomicU64>,
}

impl ProxyMetrics {
    /// Create a new metrics instance with all counters at zero.
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            rate_limited: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            total_tokens: AtomicU64::new(0),
            total_cost_cents: AtomicU64::new(0),
            per_model_requests: DashMap::new(),
        }
    }

    /// Record a request.
    pub fn record_request(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache hit.
    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache miss.
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a rate-limited request.
    pub fn record_rate_limited(&self) {
        self.rate_limited.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an error.
    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record token usage and cost.
    pub fn record_usage(&self, tokens: u64, cost_cents: u64) {
        self.total_tokens.fetch_add(tokens, Ordering::Relaxed);
        self.total_cost_cents
            .fetch_add(cost_cents, Ordering::Relaxed);
    }

    /// Record a request to a specific model.
    pub fn record_model_request(&self, model: &str) {
        self.per_model_requests
            .entry(model.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Take a serializable snapshot of current metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let mut per_model = std::collections::HashMap::new();
        for entry in self.per_model_requests.iter() {
            per_model.insert(entry.key().clone(), entry.value().load(Ordering::Relaxed));
        }

        MetricsSnapshot {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            rate_limited: self.rate_limited.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            total_tokens: self.total_tokens.load(Ordering::Relaxed),
            total_cost_cents: self.total_cost_cents.load(Ordering::Relaxed),
            per_model_requests: per_model,
        }
    }
}

impl Default for ProxyMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializable metrics snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiState)]
pub struct MetricsSnapshot {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub rate_limited: u64,
    pub errors: u64,
    pub total_tokens: u64,
    pub total_cost_cents: u64,
    pub per_model_requests: std::collections::HashMap<String, u64>,
}

impl MetricsSnapshot {
    /// Cache hit ratio (0.0 to 1.0).
    pub fn cache_hit_ratio(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }

    /// Total cost in dollars.
    pub fn total_cost_dollars(&self) -> f64 {
        self.total_cost_cents as f64 / 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_increments() {
        let m = ProxyMetrics::new();

        m.record_request();
        m.record_request();
        m.record_cache_hit();
        m.record_cache_miss();
        m.record_rate_limited();
        m.record_error();
        m.record_usage(1000, 50);

        assert_eq!(m.total_requests.load(Ordering::Relaxed), 2);
        assert_eq!(m.cache_hits.load(Ordering::Relaxed), 1);
        assert_eq!(m.cache_misses.load(Ordering::Relaxed), 1);
        assert_eq!(m.rate_limited.load(Ordering::Relaxed), 1);
        assert_eq!(m.errors.load(Ordering::Relaxed), 1);
        assert_eq!(m.total_tokens.load(Ordering::Relaxed), 1000);
        assert_eq!(m.total_cost_cents.load(Ordering::Relaxed), 50);
    }

    #[test]
    fn test_per_model_tracking() {
        let m = ProxyMetrics::new();

        m.record_model_request("gpt-4");
        m.record_model_request("gpt-4");
        m.record_model_request("claude-3");

        let snap = m.snapshot();
        assert_eq!(snap.per_model_requests.get("gpt-4"), Some(&2));
        assert_eq!(snap.per_model_requests.get("claude-3"), Some(&1));
    }

    #[test]
    fn test_snapshot() {
        let m = ProxyMetrics::new();

        m.record_request();
        m.record_cache_hit();
        m.record_cache_miss();
        m.record_usage(500, 25);
        m.record_model_request("gpt-4");

        let snap = m.snapshot();
        assert_eq!(snap.total_requests, 1);
        assert_eq!(snap.cache_hits, 1);
        assert_eq!(snap.cache_misses, 1);
        assert_eq!(snap.total_tokens, 500);
        assert_eq!(snap.total_cost_cents, 25);
        assert_eq!(snap.total_cost_dollars(), 0.25);
        assert_eq!(snap.cache_hit_ratio(), 0.5);
    }

    #[test]
    fn test_snapshot_serializable() {
        let m = ProxyMetrics::new();
        m.record_request();
        let snap = m.snapshot();
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("\"total_requests\":1"));
    }

    #[test]
    fn test_cache_hit_ratio_zero_requests() {
        let snap = MetricsSnapshot {
            total_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
            rate_limited: 0,
            errors: 0,
            total_tokens: 0,
            total_cost_cents: 0,
            per_model_requests: std::collections::HashMap::new(),
        };
        assert_eq!(snap.cache_hit_ratio(), 0.0);
    }
}
