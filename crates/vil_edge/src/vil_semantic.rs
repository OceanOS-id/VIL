use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct EdgeEvent {
    pub model_name: String,
    pub target_arch: String,
    pub latency_ms: u64,
    pub memory_used_mb: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct EdgeFault {
    pub message: String,
    pub model_name: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct EdgeState {
    pub models_loaded: u64,
    pub total_inferences: u64,
    pub total_memory_mb: u64,
}
