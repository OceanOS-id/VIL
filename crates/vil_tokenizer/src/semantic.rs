//! Semantic types for tokenizer operations (Tier B AI).

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct TokenizeEvent {
    pub text_length: u32,
    pub token_count: u32,
    pub latency_us: u64,
    pub vocab: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct TokenizeFault {
    pub message: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct TokenizerState {
    pub total_requests: u64,
    pub total_tokens_counted: u64,
}
