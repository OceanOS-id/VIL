use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct OptimizeEvent {
    pub strategy: String,
    pub candidates_evaluated: u32,
    pub best_score: f64,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct OptimizeFault {
    pub message: String,
    pub strategy: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct OptimizerState {
    pub total_optimizations: u64,
    pub total_candidates_evaluated: u64,
    pub best_overall_score: f64,
}
