use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct BenchEvent {
    pub suite_name: String,
    pub benchmarks_run: u32,
    pub total_cases: u32,
    pub avg_score: f64,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct BenchFault {
    pub message: String,
    pub benchmark: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct BenchState {
    pub total_runs: u64,
    pub total_cases_evaluated: u64,
    pub avg_overall_score: f64,
}
