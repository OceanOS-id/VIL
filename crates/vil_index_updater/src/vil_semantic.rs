use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct IndexUpdateEvent {
    pub operation: String,
    pub entries_flushed: u32,
    pub wal_size: u64,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct IndexUpdateFault {
    pub message: String,
    pub operation: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct IndexUpdaterState {
    pub total_flushes: u64,
    pub total_entries_written: u64,
    pub pending_wal_entries: u64,
}
