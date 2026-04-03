use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct SearchEvent {
    pub collection: String,
    pub top_k: u32,
    pub results_found: u32,
    pub latency_ns: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct IndexEvent {
    pub collection: String,
    pub vectors_added: u32,
    pub latency_ns: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct VectorDbFault {
    pub collection: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct VectorDbState {
    pub total_vectors: u64,
    pub total_searches: u64,
    pub collections: u32,
}
