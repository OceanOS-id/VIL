// =============================================================================
// VIL Semantic Types — Multi-Agent
// =============================================================================

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

/// Events emitted by the Multi-Agent subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiEvent)]
pub enum MultiAgentEvent {
    /// The orchestrator started a run.
    RunStarted { initial_query: String, agent_count: usize },
    /// An individual agent completed its task.
    AgentCompleted { agent_name: String, output_len: usize },
    /// A message was passed between agents.
    MessagePassed { from: String, to: String },
    /// The orchestrator completed a full run.
    RunCompleted {
        final_answer_len: usize,
        total_ms: u64,
        messages_exchanged: usize,
    },
}

/// Faults that can occur in the Multi-Agent subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiFault)]
pub enum MultiAgentFault {
    /// An agent failed during execution.
    AgentFailed { agent: String, reason: String },
    /// The graph was invalid or empty.
    InvalidGraph { reason: String },
    /// A channel send or receive failed.
    ChannelError { reason: String },
    /// The orchestrator timed out.
    Timeout { elapsed_ms: u64, limit_ms: u64 },
}

/// Observable state of the Multi-Agent subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiState)]
pub struct MultiAgentState {
    /// Number of agents in the graph.
    pub agent_count: usize,
    /// Number of edges in the graph.
    pub edge_count: usize,
    /// Configured timeout in milliseconds.
    pub timeout_ms: u64,
    /// Maximum allowed messages.
    pub max_messages: usize,
}
