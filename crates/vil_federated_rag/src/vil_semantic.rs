use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct FederatedEvent {
    pub query: String,
    pub sources_queried: u32,
    pub results_merged: u32,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct FederatedFault {
    pub message: String,
    pub source_id: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct FederatedState {
    pub total_queries: u64,
    pub total_results_returned: u64,
    pub total_source_failures: u64,
}
