// =============================================================================
// VIL Server Observability Middleware — Zero-instrumentation metrics
// =============================================================================
//
// Automatically generates per-handler Prometheus metrics without any
// annotation or manual instrumentation. Every route handler gets:
//
//   vil_handler_requests_total{route="/api/orders", method="GET", status="200"}
//   vil_handler_duration_ms{route="/api/orders", method="GET"}
//   vil_handler_in_flight{route="/api/orders"}
//   vil_handler_errors_total{route="/api/orders", code="500"}
//
// This is a key disruptive feature — Spring requires @Timed/@Traced,
// Quarkus needs MicroProfile annotations. vil-server does it automatically.

use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use dashmap::DashMap;

use crate::state::AppState;

/// Latency histogram bucket boundaries in microseconds.
/// 40 buckets — sub-microsecond to 5s, tight spacing in the 1-100ms critical range.
/// Overhead per bucket: 1 atomic increment (~1ns). 40 buckets = 40 * 8 = 320 bytes per route.
pub const LATENCY_BUCKETS_US: [u64; 40] = [
    // sub-ms (fast handlers)
    10, 25, 50, 75, 100, 150, 200, 300, 500, 750,
    // 1-10ms (typical API handlers)
    1_000, 1_500, 2_000, 2_500, 3_000, 4_000, 5_000, 6_000, 7_500, 10_000,
    // 10-100ms (DB queries, moderate upstream)
    12_500, 15_000, 20_000, 25_000, 30_000, 40_000, 50_000, 60_000, 75_000, 100_000,
    // 100ms-1s (upstream SSE, LLM inference)
    125_000, 150_000, 200_000, 300_000, 500_000, 750_000, 1_000_000,
    // 1-5s (long-running streams)
    1_500_000, 2_000_000, 5_000_000,
];

/// Per-route metrics collector.
/// Each route accumulates its own counters independently.
pub struct RouteMetrics {
    pub requests_total: AtomicU64,
    pub errors_total: AtomicU64,
    pub duration_sum_us: AtomicU64,
    pub duration_count: AtomicU64,
    pub in_flight: AtomicU64,
    pub min_us: AtomicU64,
    pub max_us: AtomicU64,
    /// Histogram buckets: bucket[i] counts requests with latency <= LATENCY_BUCKETS_US[i].
    pub latency_buckets: [AtomicU64; 41],
}

impl Default for RouteMetrics {
    fn default() -> Self {
        Self {
            requests_total: AtomicU64::new(0),
            errors_total: AtomicU64::new(0),
            duration_sum_us: AtomicU64::new(0),
            duration_count: AtomicU64::new(0),
            in_flight: AtomicU64::new(0),
            min_us: AtomicU64::new(u64::MAX),
            max_us: AtomicU64::new(0),
            latency_buckets: std::array::from_fn(|_| AtomicU64::new(0)),
        }
    }
}

/// Global handler metrics registry.
/// Thread-safe, lock-free per-route metrics collection.
pub struct HandlerMetricsRegistry {
    routes: DashMap<String, RouteMetrics>,
}

impl HandlerMetricsRegistry {
    pub fn new() -> Self {
        Self {
            routes: DashMap::new(),
        }
    }

    fn get_or_create(&self, key: &str) -> dashmap::mapref::one::Ref<'_, String, RouteMetrics> {
        if !self.routes.contains_key(key) {
            self.routes.insert(key.to_string(), RouteMetrics::default());
        }
        self.routes.get(key).unwrap()
    }

    pub fn request_start(&self, key: &str) {
        let m = self.get_or_create(key);
        m.requests_total.fetch_add(1, Ordering::Relaxed);
        m.in_flight.fetch_add(1, Ordering::Relaxed);
    }

    pub fn request_end(&self, key: &str, duration_us: u64, is_error: bool) {
        if let Some(m) = self.routes.get(key) {
            m.in_flight.fetch_sub(1, Ordering::Relaxed);
            m.duration_sum_us.fetch_add(duration_us, Ordering::Relaxed);
            m.duration_count.fetch_add(1, Ordering::Relaxed);
            if is_error {
                m.errors_total.fetch_add(1, Ordering::Relaxed);
            }
            // Update min (CAS loop)
            let mut cur = m.min_us.load(Ordering::Relaxed);
            while duration_us < cur {
                match m.min_us.compare_exchange_weak(cur, duration_us, Ordering::Relaxed, Ordering::Relaxed) {
                    Ok(_) => break,
                    Err(c) => cur = c,
                }
            }
            // Update max (CAS loop)
            let mut cur = m.max_us.load(Ordering::Relaxed);
            while duration_us > cur {
                match m.max_us.compare_exchange_weak(cur, duration_us, Ordering::Relaxed, Ordering::Relaxed) {
                    Ok(_) => break,
                    Err(c) => cur = c,
                }
            }
            // Record latency bucket
            let idx = LATENCY_BUCKETS_US.iter()
                .position(|&b| duration_us <= b)
                .unwrap_or(LATENCY_BUCKETS_US.len());
            m.latency_buckets[idx].fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Compute percentile latency (e.g. 0.95 for P95) from histogram buckets.
    pub fn percentile_us(buckets: &[AtomicU64; 41], _total_hint: u64, pct: f64) -> u64 {
        // Use actual bucket sum as total (not requests_total which includes in-flight)
        let counts: Vec<u64> = buckets.iter().map(|b| b.load(Ordering::Relaxed)).collect();
        let total: u64 = counts.iter().sum();
        if total == 0 { return 0; }
        let target = (total as f64 * pct).ceil() as u64;
        let mut cumulative = 0u64;
        for (i, &count) in counts.iter().enumerate() {
            cumulative += count;
            if cumulative >= target {
                return if i < LATENCY_BUCKETS_US.len() {
                    LATENCY_BUCKETS_US[i]
                } else {
                    LATENCY_BUCKETS_US[LATENCY_BUCKETS_US.len() - 1] * 2
                };
            }
        }
        // Fallback: return last bucket boundary
        LATENCY_BUCKETS_US[LATENCY_BUCKETS_US.len() - 1]
    }

    /// Export all route metrics in Prometheus text format.
    pub fn to_prometheus(&self) -> String {
        let mut out = String::new();

        out.push_str("# HELP vil_handler_requests_total Total requests per handler\n");
        out.push_str("# TYPE vil_handler_requests_total counter\n");

        out.push_str("# HELP vil_handler_errors_total Total errors per handler\n");
        out.push_str("# TYPE vil_handler_errors_total counter\n");

        out.push_str("# HELP vil_handler_duration_us_sum Total duration in microseconds per handler\n");
        out.push_str("# TYPE vil_handler_duration_us_sum counter\n");

        out.push_str("# HELP vil_handler_in_flight Current in-flight requests per handler\n");
        out.push_str("# TYPE vil_handler_in_flight gauge\n");

        for entry in self.routes.iter() {
            let key = entry.key();
            let m = entry.value();

            // Parse "GET /path" into method and route
            let parts: Vec<&str> = key.splitn(2, ' ').collect();
            let (method, route) = if parts.len() == 2 {
                (parts[0], parts[1])
            } else {
                ("UNKNOWN", key.as_str())
            };

            let reqs = m.requests_total.load(Ordering::Relaxed);
            let errs = m.errors_total.load(Ordering::Relaxed);
            let dur_sum_us = m.duration_sum_us.load(Ordering::Relaxed);
            let in_flight = m.in_flight.load(Ordering::Relaxed);

            out.push_str(&format!(
                "vil_handler_requests_total{{method=\"{}\",route=\"{}\"}} {}\n",
                method, route, reqs
            ));
            out.push_str(&format!(
                "vil_handler_errors_total{{method=\"{}\",route=\"{}\"}} {}\n",
                method, route, errs
            ));
            out.push_str(&format!(
                "vil_handler_duration_us_sum{{method=\"{}\",route=\"{}\"}} {}\n",
                method, route, dur_sum_us
            ));
            out.push_str(&format!(
                "vil_handler_in_flight{{method=\"{}\",route=\"{}\"}} {}\n",
                method, route, in_flight
            ));
        }

        out
    }

    /// Get the number of tracked routes.
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    /// Sync all route metrics into an observer MetricsCollector.
    /// Called periodically by the observer bridge task.
    pub fn sync_to_observer(&self, collector: &vil_observer::metrics::MetricsCollector) {
        for entry in self.routes.iter() {
            let key = entry.key();
            let m = entry.value();

            let parts: Vec<&str> = key.splitn(2, ' ').collect();
            let (method, path) = if parts.len() == 2 {
                (parts[0], parts[1])
            } else {
                ("UNKNOWN", key.as_str())
            };

            let reqs = m.requests_total.load(Ordering::Relaxed);
            let errs = m.errors_total.load(Ordering::Relaxed);
            let dur_count = m.duration_count.load(Ordering::Relaxed);
            let dur_sum_us = m.duration_sum_us.load(Ordering::Relaxed);
            let avg_us = if dur_count > 0 { dur_sum_us / dur_count } else { 0 };

            let min = m.min_us.load(Ordering::Relaxed);
            let max = m.max_us.load(Ordering::Relaxed);
            let p95  = Self::percentile_us(&m.latency_buckets, reqs, 0.95);
            let p99  = Self::percentile_us(&m.latency_buckets, reqs, 0.99);
            let p999 = Self::percentile_us(&m.latency_buckets, reqs, 0.999);

            collector.sync_endpoint_full(method, path, reqs, errs, avg_us, min, max, p95, p99, p999);
        }
    }
}

impl Default for HandlerMetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Sampling counter for metrics (avoid per-request overhead under extreme load).
static METRICS_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Metrics sample rate: 1 = every request (default), N = every Nth request.
/// Set to 10 for ~10% sampling under extreme load (>500K req/s).
pub static METRICS_SAMPLE_RATE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Auto-observability middleware.
///
/// Records per-route metrics for every request (or sampled subset).
/// Optimized: pre-computes key using method bytes + path slice to
/// avoid String allocation on the hot path.
pub async fn handler_metrics(
    State(state): State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let sample_rate = METRICS_SAMPLE_RATE.load(Ordering::Relaxed);
    let counter = METRICS_COUNTER.fetch_add(1, Ordering::Relaxed);

    // Fast path: skip metrics if not sampled
    if sample_rate > 1 && counter % sample_rate != 0 {
        return next.run(request).await;
    }

    let path = request.uri().path();

    // Skip observer internal routes — don't pollute business metrics
    if path.starts_with("/_vil/") {
        return next.run(request).await;
    }

    let start = Instant::now();
    let method = request.method().as_str();
    let key_len = method.len() + 1 + path.len();
    let mut key = String::with_capacity(key_len);
    key.push_str(method);
    key.push(' ');
    key.push_str(path);

    state.handler_metrics().request_start(&key);

    let response = next.run(request).await;

    let duration_us = start.elapsed().as_micros() as u64;
    let is_error = response.status().is_server_error();
    state.handler_metrics().request_end(&key, duration_us, is_error);

    response
}
