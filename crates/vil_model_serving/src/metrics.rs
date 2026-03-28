use serde::{Deserialize, Serialize};
use vil_macros::VilAiState;

/// Per-variant quality and performance metrics.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiState)]
pub struct VariantMetrics {
    /// Total requests served by this variant.
    pub requests: u64,
    /// Total errors encountered.
    pub errors: u64,
    /// Running average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Running average quality score (caller-supplied, 0.0 – 1.0).
    pub avg_quality_score: f64,
}

impl Default for VariantMetrics {
    fn default() -> Self {
        Self {
            requests: 0,
            errors: 0,
            avg_latency_ms: 0.0,
            avg_quality_score: 0.0,
        }
    }
}

impl VariantMetrics {
    /// Record a successful request with its latency.
    pub fn record_request(&mut self, latency_ms: u64) {
        self.requests += 1;
        // Incremental average
        self.avg_latency_ms += (latency_ms as f64 - self.avg_latency_ms) / self.requests as f64;
    }

    /// Record an error.
    pub fn record_error(&mut self) {
        self.errors += 1;
    }

    /// Record a quality score (0.0 – 1.0).
    pub fn record_quality(&mut self, score: f64) {
        let quality_count = self.requests; // quality is tracked per-request
        if quality_count == 0 {
            self.avg_quality_score = score;
        } else {
            self.avg_quality_score += (score - self.avg_quality_score) / quality_count as f64;
        }
    }

    /// Error rate as a fraction.
    pub fn error_rate(&self) -> f64 {
        if self.requests == 0 {
            0.0
        } else {
            self.errors as f64 / self.requests as f64
        }
    }
}
