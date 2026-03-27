// =============================================================================
// VIL Server — Custom Metrics Registration API
// =============================================================================
//
// Allows handlers to register and update custom Prometheus metrics.
// Beyond auto-generated per-handler metrics, users can create:
//   - Counters (monotonic increment)
//   - Gauges (arbitrary value)
//   - Histograms (value distribution)
//
// All custom metrics are exported alongside auto-metrics at /metrics.

use dashmap::DashMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;

/// Metric type.
#[derive(Debug, Clone, Copy)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
}

/// Custom metrics registry.
pub struct CustomMetrics {
    counters: DashMap<String, Arc<AtomicU64>>,
    gauges: DashMap<String, Arc<AtomicI64>>,
    histograms: DashMap<String, Arc<HistogramData>>,
    descriptions: DashMap<String, (MetricType, String)>,
}

/// Histogram data with fixed buckets.
pub struct HistogramData {
    /// Bucket boundaries
    boundaries: Vec<f64>,
    /// Count per bucket
    buckets: Vec<AtomicU64>,
    /// Total sum of observed values
    sum: AtomicU64,
    /// Total count of observations
    count: AtomicU64,
}

impl HistogramData {
    fn new(boundaries: Vec<f64>) -> Self {
        let bucket_count = boundaries.len() + 1; // +1 for +Inf
        let buckets = (0..bucket_count).map(|_| AtomicU64::new(0)).collect();
        Self {
            boundaries,
            buckets,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    fn observe(&self, value: f64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum.fetch_add(value.to_bits(), Ordering::Relaxed);

        for (i, boundary) in self.boundaries.iter().enumerate() {
            if value <= *boundary {
                self.buckets[i].fetch_add(1, Ordering::Relaxed);
                return;
            }
        }
        // +Inf bucket
        if let Some(last) = self.buckets.last() {
            last.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl CustomMetrics {
    pub fn new() -> Self {
        Self {
            counters: DashMap::new(),
            gauges: DashMap::new(),
            histograms: DashMap::new(),
            descriptions: DashMap::new(),
        }
    }

    // ========== Counters ==========

    /// Register a counter metric.
    pub fn register_counter(&self, name: &str, description: &str) {
        self.counters.insert(name.to_string(), Arc::new(AtomicU64::new(0)));
        self.descriptions.insert(
            name.to_string(),
            (MetricType::Counter, description.to_string()),
        );
    }

    /// Increment a counter by 1.
    pub fn inc(&self, name: &str) {
        if let Some(counter) = self.counters.get(name) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Increment a counter by a specific amount.
    pub fn inc_by(&self, name: &str, amount: u64) {
        if let Some(counter) = self.counters.get(name) {
            counter.fetch_add(amount, Ordering::Relaxed);
        }
    }

    /// Get counter value.
    pub fn counter_value(&self, name: &str) -> u64 {
        self.counters.get(name).map(|c| c.load(Ordering::Relaxed)).unwrap_or(0)
    }

    // ========== Gauges ==========

    /// Register a gauge metric.
    pub fn register_gauge(&self, name: &str, description: &str) {
        self.gauges.insert(name.to_string(), Arc::new(AtomicI64::new(0)));
        self.descriptions.insert(
            name.to_string(),
            (MetricType::Gauge, description.to_string()),
        );
    }

    /// Set gauge value.
    pub fn gauge_set(&self, name: &str, value: i64) {
        if let Some(gauge) = self.gauges.get(name) {
            gauge.store(value, Ordering::Relaxed);
        }
    }

    /// Increment gauge.
    pub fn gauge_inc(&self, name: &str) {
        if let Some(gauge) = self.gauges.get(name) {
            gauge.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Decrement gauge.
    pub fn gauge_dec(&self, name: &str) {
        if let Some(gauge) = self.gauges.get(name) {
            gauge.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Get gauge value.
    pub fn gauge_value(&self, name: &str) -> i64 {
        self.gauges.get(name).map(|g| g.load(Ordering::Relaxed)).unwrap_or(0)
    }

    // ========== Histograms ==========

    /// Register a histogram with custom bucket boundaries.
    pub fn register_histogram(&self, name: &str, description: &str, buckets: Vec<f64>) {
        self.histograms.insert(name.to_string(), Arc::new(HistogramData::new(buckets)));
        self.descriptions.insert(
            name.to_string(),
            (MetricType::Histogram, description.to_string()),
        );
    }

    /// Register a histogram with default buckets (HTTP latency).
    pub fn register_histogram_default(&self, name: &str, description: &str) {
        let buckets = vec![5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0];
        self.register_histogram(name, description, buckets);
    }

    /// Observe a value in a histogram.
    pub fn observe(&self, name: &str, value: f64) {
        if let Some(h) = self.histograms.get(name) {
            h.observe(value);
        }
    }

    // ========== Export ==========

    /// Export all custom metrics in Prometheus text format.
    pub fn to_prometheus(&self) -> String {
        let mut out = String::new();

        // Counters
        for entry in self.counters.iter() {
            let name = entry.key();
            let value = entry.value().load(Ordering::Relaxed);
            if let Some(desc) = self.descriptions.get(name) {
                out.push_str(&format!("# HELP {} {}\n", name, desc.1));
                out.push_str(&format!("# TYPE {} counter\n", name));
            }
            out.push_str(&format!("{} {}\n", name, value));
        }

        // Gauges
        for entry in self.gauges.iter() {
            let name = entry.key();
            let value = entry.value().load(Ordering::Relaxed);
            if let Some(desc) = self.descriptions.get(name) {
                out.push_str(&format!("# HELP {} {}\n", name, desc.1));
                out.push_str(&format!("# TYPE {} gauge\n", name));
            }
            out.push_str(&format!("{} {}\n", name, value));
        }

        // Histograms
        for entry in self.histograms.iter() {
            let name = entry.key();
            let h = entry.value();
            if let Some(desc) = self.descriptions.get(name) {
                out.push_str(&format!("# HELP {} {}\n", name, desc.1));
                out.push_str(&format!("# TYPE {} histogram\n", name));
            }
            let count = h.count.load(Ordering::Relaxed);
            out.push_str(&format!("{}_count {}\n", name, count));
            out.push_str(&format!("{}_sum {}\n", name, f64::from_bits(h.sum.load(Ordering::Relaxed))));
        }

        out
    }

    /// Get total number of registered metrics.
    pub fn metric_count(&self) -> usize {
        self.counters.len() + self.gauges.len() + self.histograms.len()
    }
}

impl Default for CustomMetrics {
    fn default() -> Self {
        Self::new()
    }
}
