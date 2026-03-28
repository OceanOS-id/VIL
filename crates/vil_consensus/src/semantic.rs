//! Semantic types for multi-model consensus operations.
//!
//! VIL process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after a consensus query completes.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct ConsensusEvent {
    pub strategy: String,
    pub provider_count: u32,
    pub successful_providers: u32,
    pub winning_model: String,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of consensus failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsensusFaultType {
    AllProvidersFailed,
    TimeoutExceeded,
    StrategyFailed,
}

/// Emitted when a consensus operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct ConsensusFault {
    pub error_type: ConsensusFaultType,
    pub message: String,
}

impl ConsensusFault {
    pub fn all_providers_failed(msg: impl Into<String>) -> Self {
        Self {
            error_type: ConsensusFaultType::AllProvidersFailed,
            message: msg.into(),
        }
    }

    pub fn timeout_exceeded(msg: impl Into<String>) -> Self {
        Self {
            error_type: ConsensusFaultType::TimeoutExceeded,
            message: msg.into(),
        }
    }

    pub fn strategy_failed(msg: impl Into<String>) -> Self {
        Self {
            error_type: ConsensusFaultType::StrategyFailed,
            message: msg.into(),
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks the current state of the consensus engine.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct ConsensusState {
    pub total_queries: u64,
    pub total_provider_calls: u64,
    pub total_failures: u64,
    pub avg_successful_providers: f32,
}
