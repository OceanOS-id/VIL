use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct SyntheticEvent {
    pub template: String,
    pub examples_generated: u32,
    pub quality_pass_rate: f64,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct SyntheticFault {
    pub message: String,
    pub template: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct SyntheticState {
    pub total_requests: u64,
    pub total_examples_generated: u64,
    pub total_quality_failures: u64,
}
