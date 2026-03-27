// gRPC per-service metrics.

use std::sync::atomic::{AtomicU64, Ordering};
use dashmap::DashMap;

/// Per-method gRPC metrics.
pub struct GrpcMetrics {
    methods: DashMap<String, MethodMetrics>,
}

struct MethodMetrics {
    requests: AtomicU64,
    errors: AtomicU64,
    duration_sum_us: AtomicU64,
}

impl GrpcMetrics {
    pub fn new() -> Self { Self { methods: DashMap::new() } }

    pub fn record(&self, method: &str, duration_us: u64, is_error: bool) {
        let entry = self.methods.entry(method.to_string()).or_insert_with(|| MethodMetrics {
            requests: AtomicU64::new(0), errors: AtomicU64::new(0), duration_sum_us: AtomicU64::new(0),
        });
        entry.requests.fetch_add(1, Ordering::Relaxed);
        entry.duration_sum_us.fetch_add(duration_us, Ordering::Relaxed);
        if is_error { entry.errors.fetch_add(1, Ordering::Relaxed); }
    }

    pub fn to_prometheus(&self) -> String {
        let mut out = String::new();
        for entry in self.methods.iter() {
            let m = entry.value();
            out.push_str(&format!(
                "vil_grpc_requests_total{{method=\"{}\"}} {}\n\
                 vil_grpc_errors_total{{method=\"{}\"}} {}\n",
                entry.key(), m.requests.load(Ordering::Relaxed),
                entry.key(), m.errors.load(Ordering::Relaxed),
            ));
        }
        out
    }

    pub fn method_count(&self) -> usize { self.methods.len() }
}

impl Default for GrpcMetrics { fn default() -> Self { Self::new() } }
