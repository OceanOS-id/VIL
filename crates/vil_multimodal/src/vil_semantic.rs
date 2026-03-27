use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct MultimodalEvent {
    pub operation: String,
    pub modalities: Vec<String>,
    pub dimension: usize,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct MultimodalFault {
    pub message: String,
    pub operation: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct MultimodalState {
    pub total_fusions: u64,
    pub total_searches: u64,
    pub total_errors: u64,
}
