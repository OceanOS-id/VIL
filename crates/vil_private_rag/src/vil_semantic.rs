use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct PrivateRagEvent {
    pub operation: String,
    pub items_processed: u32,
    pub pii_detected: u32,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct PrivateRagFault {
    pub message: String,
    pub operation: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct PrivateRagState {
    pub total_redactions: u64,
    pub total_anonymizations: u64,
    pub total_audit_entries: u64,
}
