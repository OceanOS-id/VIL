use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct PromptRenderEvent {
    pub template_name: String,
    pub variable_count: u32,
    pub output_length: u64,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct PromptFault {
    pub message: String,
    pub template_name: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct PromptsState {
    pub total_renders: u64,
    pub templates_registered: u64,
}
