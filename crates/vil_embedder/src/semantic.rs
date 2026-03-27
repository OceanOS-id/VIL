use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct EmbedEvent {
    pub text_count: u32,
    pub dimension: u32,
    pub latency_ms: u64,
    pub provider: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct EmbedFault {
    pub message: String,
    pub provider: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct EmbedderState {
    pub total_requests: u64,
    pub total_texts_embedded: u64,
}
