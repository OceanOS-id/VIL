//! Semantic types for Agent operations.
//!
//! These types follow VIL's process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)
//! - Decisions: routing/control choices (Control Lane)
//!
//! Each type uses Tier B AI semantic derive macros so the VIL runtime
//! can route them to the correct tri-lane automatically.

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiDecision, VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after each tool invocation inside the ReAct loop.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct AgentToolCallEvent {
    pub tool_name: String,
    pub input_summary: String,
    pub output_summary: String,
    pub duration_ms: u64,
    pub success: bool,
}

/// Emitted when the agent finishes processing a query.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct AgentCompletionEvent {
    pub query_summary: String,
    pub answer_length: u32,
    pub tools_used: Vec<String>,
    pub iterations: u32,
    pub total_ms: u64,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of agent failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentFaultType {
    ToolFailed,
    MaxIterations,
    LlmFailed,
}

/// Emitted when the agent encounters an error.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct AgentFault {
    pub error_type: AgentFaultType,
    pub tool_name: Option<String>,
    pub message: String,
}

impl AgentFault {
    /// Convenience constructor for tool failures.
    pub fn tool_failed(tool: &str, msg: impl Into<String>) -> Self {
        Self {
            error_type: AgentFaultType::ToolFailed,
            tool_name: Some(tool.into()),
            message: msg.into(),
        }
    }

    /// Convenience constructor for max-iteration breaches.
    pub fn max_iterations(iterations: u32) -> Self {
        Self {
            error_type: AgentFaultType::MaxIterations,
            tool_name: None,
            message: format!("exceeded max iterations ({iterations})"),
        }
    }

    /// Convenience constructor for LLM failures inside the agent loop.
    pub fn llm_failed(msg: impl Into<String>) -> Self {
        Self {
            error_type: AgentFaultType::LlmFailed,
            tool_name: None,
            message: msg.into(),
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks the current agent memory/context state.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct AgentMemoryState {
    pub messages_count: u32,
    pub context_tokens: u64,
    pub tools_available: Vec<String>,
}

// ── Decisions (Control Lane, routing choices) ───────────────────────

/// The next action the agent has decided to take.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentNextAction {
    CallTool,
    FinalAnswer,
}

/// Represents the agent's routing decision at each ReAct iteration.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiDecision)]
pub struct AgentRoutingDecision {
    pub next_action: AgentNextAction,
    pub tool_name: Option<String>,
    pub confidence: f64,
}
