use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct InferEvent {
    pub model: String,
    pub batch_size: u32,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct InferFault {
    pub model: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct InferState {
    pub total_requests: u64,
    pub total_batches: u64,
    pub models_loaded: u32,
}
