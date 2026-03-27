use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct LayoutAnalyzeEvent {
    pub region_count: u32,
    pub section_count: u32,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct LayoutFault {
    pub message: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct DocLayoutState {
    pub total_analyses: u64,
    pub total_regions_detected: u64,
}
