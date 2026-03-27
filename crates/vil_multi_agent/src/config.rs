//! Configuration for the multi-agent system.

use serde::{Deserialize, Serialize};

/// Configuration for the multi-agent orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiAgentConfig {
    /// Maximum time (ms) the orchestrator is allowed to run before aborting.
    pub timeout_ms: u64,
    /// Maximum number of messages that can be exchanged before the run is stopped.
    pub max_messages: usize,
    /// Whether to collect per-agent output in the final result.
    pub collect_agent_outputs: bool,
}

impl Default for MultiAgentConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 30_000,
            max_messages: 1_000,
            collect_agent_outputs: true,
        }
    }
}
