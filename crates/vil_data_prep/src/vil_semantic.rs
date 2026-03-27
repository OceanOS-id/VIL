use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct DataPrepEvent {
    pub step: String,
    pub records_in: u64,
    pub records_out: u64,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct DataPrepFault {
    pub message: String,
    pub step: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct DataPrepState {
    pub total_requests: u64,
    pub total_records_processed: u64,
    pub total_records_output: u64,
}
