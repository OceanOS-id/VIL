//! AI-specific metrics aggregated from spans.

use serde::{Deserialize, Serialize};
use vil_macros::VilAiState;

/// Aggregated AI metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize, VilAiState)]
pub struct AiMetrics {
    pub total_llm_calls: u64,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub avg_latency_ms: f64,
    pub cache_hit_rate: f64,
    pub total_spans: u64,
    pub error_count: u64,
}
