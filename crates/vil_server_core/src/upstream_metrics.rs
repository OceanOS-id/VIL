// =============================================================================
// VIL Upstream Metrics — lock-free per-upstream latency tracking
// =============================================================================
//
// Tracks outbound HTTP calls to upstream services (LLM providers, databases,
// external APIs). Feeds into the observer dashboard "Upstreams" panel.

use dashmap::DashMap;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use crate::obs_middleware::LATENCY_BUCKETS_US;

// ── Global singleton ──────────────────────────────────────────────────────────

static GLOBAL_REGISTRY: OnceLock<Arc<UpstreamRegistry>> = OnceLock::new();
static ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Enable upstream metrics tracking (called when observer is ON).
pub fn enable() {
    ENABLED.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Get the global upstream registry (singleton, created on first access).
pub fn global() -> &'static Arc<UpstreamRegistry> {
    GLOBAL_REGISTRY.get_or_init(|| Arc::new(UpstreamRegistry::new()))
}

/// Record an upstream call (no-op when observer OFF).
pub fn record_start(url: &str) {
    if !ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    global().call_start(url);
}

/// Record upstream call completion (no-op when observer OFF).
pub fn record_end(url: &str, duration_us: u64, status: u16, is_error: bool) {
    if !ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    global().call_end(url, duration_us, status, is_error);
}

/// Per-upstream atomic metrics.
pub struct UpstreamEndpoint {
    pub url: String,
    pub requests: AtomicU64,
    pub errors: AtomicU64,
    pub duration_sum_us: AtomicU64,
    pub duration_count: AtomicU64,
    pub in_flight: AtomicU64,
    pub latency_buckets: [AtomicU64; 41],
    pub last_status: AtomicU64,
}

impl UpstreamEndpoint {
    fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            requests: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            duration_sum_us: AtomicU64::new(0),
            duration_count: AtomicU64::new(0),
            in_flight: AtomicU64::new(0),
            latency_buckets: std::array::from_fn(|_| AtomicU64::new(0)),
            last_status: AtomicU64::new(0),
        }
    }
}

/// Global upstream metrics registry. Thread-safe, lock-free.
pub struct UpstreamRegistry {
    upstreams: DashMap<String, Arc<UpstreamEndpoint>>,
}

impl UpstreamRegistry {
    pub fn new() -> Self {
        Self {
            upstreams: DashMap::new(),
        }
    }

    /// Record start of an upstream call.
    pub fn call_start(&self, url: &str) {
        let entry = self
            .upstreams
            .entry(url.to_string())
            .or_insert_with(|| Arc::new(UpstreamEndpoint::new(url)));
        entry.requests.fetch_add(1, Ordering::Relaxed);
        entry.in_flight.fetch_add(1, Ordering::Relaxed);
    }

    /// Record end of an upstream call.
    pub fn call_end(&self, url: &str, duration_us: u64, status: u16, is_error: bool) {
        if let Some(m) = self.upstreams.get(url) {
            m.in_flight.fetch_sub(1, Ordering::Relaxed);
            m.duration_sum_us.fetch_add(duration_us, Ordering::Relaxed);
            m.duration_count.fetch_add(1, Ordering::Relaxed);
            m.last_status.store(status as u64, Ordering::Relaxed);
            if is_error {
                m.errors.fetch_add(1, Ordering::Relaxed);
            }
            // Histogram bucket
            let idx = LATENCY_BUCKETS_US
                .iter()
                .position(|&b| duration_us <= b)
                .unwrap_or(LATENCY_BUCKETS_US.len());
            m.latency_buckets[idx].fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get all upstream snapshots for observer dashboard.
    pub fn all_snapshots(&self) -> Vec<UpstreamSnapshot> {
        self.upstreams
            .iter()
            .map(|entry| {
                let m = entry.value();
                let reqs = m.requests.load(Ordering::Relaxed);
                let errs = m.errors.load(Ordering::Relaxed);
                let dur_count = m.duration_count.load(Ordering::Relaxed);
                let dur_sum = m.duration_sum_us.load(Ordering::Relaxed);
                let avg_us = if dur_count > 0 {
                    dur_sum / dur_count
                } else {
                    0
                };

                let p95 = crate::obs_middleware::HandlerMetricsRegistry::percentile_us(
                    &m.latency_buckets,
                    reqs,
                    0.95,
                );
                let p99 = crate::obs_middleware::HandlerMetricsRegistry::percentile_us(
                    &m.latency_buckets,
                    reqs,
                    0.99,
                );
                let p999 = crate::obs_middleware::HandlerMetricsRegistry::percentile_us(
                    &m.latency_buckets,
                    reqs,
                    0.999,
                );

                UpstreamSnapshot {
                    url: m.url.clone(),
                    requests: reqs,
                    errors: errs,
                    error_rate: if reqs > 0 {
                        errs as f64 / reqs as f64
                    } else {
                        0.0
                    },
                    avg_latency_us: avg_us,
                    p95_us: p95,
                    p99_us: p99,
                    p999_us: p999,
                    in_flight: m.in_flight.load(Ordering::Relaxed),
                    last_status: m.last_status.load(Ordering::Relaxed) as u16,
                }
            })
            .collect()
    }
}

impl Default for UpstreamRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializable upstream snapshot.
#[derive(Debug, Clone, Serialize)]
pub struct UpstreamSnapshot {
    pub url: String,
    pub requests: u64,
    pub errors: u64,
    pub error_rate: f64,
    pub avg_latency_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
    pub p999_us: u64,
    pub in_flight: u64,
    pub last_status: u16,
}
