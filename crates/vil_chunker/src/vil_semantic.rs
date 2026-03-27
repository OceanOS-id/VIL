use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct ChunkEvent {
    pub strategy: String,
    pub chunk_count: u32,
    pub total_tokens: u64,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct ChunkFault {
    pub message: String,
    pub strategy: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct ChunkerState {
    pub total_requests: u64,
    pub total_chunks_produced: u64,
}
