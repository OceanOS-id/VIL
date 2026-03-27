use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct ParseEvent {
    pub file_type: String,
    pub section_count: u32,
    pub byte_size: u64,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct ParseFault {
    pub message: String,
    pub file_type: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct DocParserState {
    pub total_documents_parsed: u64,
    pub total_sections_extracted: u64,
}
