use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct GuardrailCheckEvent {
    pub passed: bool,
    pub violation_count: u32,
    pub pii_count: u32,
    pub toxicity_score: f32,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct GuardrailFault {
    pub message: String,
    pub check_type: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct GuardrailsState {
    pub total_checks: u64,
    pub total_violations: u64,
    pub total_pii_detected: u64,
}
