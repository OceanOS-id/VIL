use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct EvalRunEvent {
    pub dataset_size: u32,
    pub metric_count: u32,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct EvalFault {
    pub message: String,
    pub metric_name: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct EvalState {
    pub total_evaluations: u64,
    pub total_cases_processed: u64,
}
