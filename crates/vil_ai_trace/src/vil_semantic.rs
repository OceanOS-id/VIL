use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct TraceEvent {
    pub trace_id: String,
    pub operation: String,
    pub spans_collected: u32,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct TraceFault {
    pub message: String,
    pub trace_id: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct TraceState {
    pub total_spans: u64,
    pub total_traces: u64,
    pub total_errors: u64,
}
