//! Trace span types for AI operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use vil_macros::VilAiEvent;

/// Type of AI operation being traced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiOperation {
    LlmCall,
    Embedding,
    Retrieval,
    Rerank,
    ToolCall,
    AgentStep,
}

impl std::fmt::Display for AiOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LlmCall => write!(f, "llm_call"),
            Self::Embedding => write!(f, "embedding"),
            Self::Retrieval => write!(f, "retrieval"),
            Self::Rerank => write!(f, "rerank"),
            Self::ToolCall => write!(f, "tool_call"),
            Self::AgentStep => write!(f, "agent_step"),
        }
    }
}

/// Status of a span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanStatus {
    Running,
    Ok,
    Error,
}

/// A single trace span.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiEvent)]
pub struct TraceSpan {
    pub trace_id: String,
    pub span_id: String,
    pub parent_id: Option<String>,
    pub operation: AiOperation,
    pub start_ms: u64,
    pub end_ms: Option<u64>,
    pub attributes: HashMap<String, String>,
    pub status: SpanStatus,
}

impl TraceSpan {
    pub fn duration_ms(&self) -> Option<u64> {
        self.end_ms.map(|e| e.saturating_sub(self.start_ms))
    }
}
