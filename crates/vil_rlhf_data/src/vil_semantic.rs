use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct RlhfEvent {
    pub operation: String,
    pub pairs_count: u32,
    pub format: String,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct RlhfFault {
    pub message: String,
    pub operation: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct RlhfState {
    pub total_requests: u64,
    pub total_pairs_processed: u64,
    pub total_exports: u64,
}
