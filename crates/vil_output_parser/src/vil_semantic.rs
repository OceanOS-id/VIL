use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct ParseEvent {
    pub parser_type: String,
    pub input_length: usize,
    pub success: bool,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct ParseFault {
    pub message: String,
    pub parser_type: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct ParserState {
    pub total_parses: u64,
    pub total_failures: u64,
    pub total_repairs: u64,
}
