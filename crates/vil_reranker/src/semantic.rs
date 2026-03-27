use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct RerankEvent {
    pub strategy: String,
    pub candidate_count: u32,
    pub top_k: u32,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct RerankFault {
    pub message: String,
    pub strategy: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct RerankerState {
    pub total_rerank_requests: u64,
    pub total_candidates_processed: u64,
}
