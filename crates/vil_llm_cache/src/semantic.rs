use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct CacheHitEvent {
    pub hit_type: String,
    pub query_hash: u64,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct CacheFault {
    pub message: String,
    pub operation: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct LlmCacheState {
    pub total_lookups: u64,
    pub total_hits: u64,
    pub total_misses: u64,
}
