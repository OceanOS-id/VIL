//! Semantic types for LLM proxy operations (Tier B AI).
use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct ProxyRequestEvent {
    pub model: String,
    pub provider: String,
    pub latency_ms: u64,
    pub cached: bool,
    pub tokens: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct ProxyFault {
    pub message: String,
    pub provider: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct ProxyState {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub rate_limited: u64,
}
