//! Semantic types for AI compiler operations.
//!
//! VIL process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after a pipeline compilation completes.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct CompileEvent {
    pub dag_nodes: u32,
    pub dag_edges: u32,
    pub compiled_steps: u32,
    pub parallel_tiers: u32,
    pub fused_transforms: u32,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of AI compiler failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CompileFaultType {
    CycleDetected,
    MissingNode,
    ExecutionFailed,
}

/// Emitted when a compiler operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct CompileFault {
    pub error_type: CompileFaultType,
    pub message: String,
}

impl CompileFault {
    pub fn cycle_detected(msg: impl Into<String>) -> Self {
        Self {
            error_type: CompileFaultType::CycleDetected,
            message: msg.into(),
        }
    }

    pub fn missing_node(msg: impl Into<String>) -> Self {
        Self {
            error_type: CompileFaultType::MissingNode,
            message: msg.into(),
        }
    }

    pub fn execution_failed(msg: impl Into<String>) -> Self {
        Self {
            error_type: CompileFaultType::ExecutionFailed,
            message: msg.into(),
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks the current state of the AI compiler.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct CompilerState {
    pub total_compilations: u64,
    pub total_executions: u64,
    pub avg_step_count: f32,
    pub avg_tier_count: f32,
}
