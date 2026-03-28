//! Semantic types for speculative decoding operations.
//!
//! VIL process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after a speculative decode run completes.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct SpeculativeEvent {
    pub draft_tokens: u32,
    pub accepted_tokens: u32,
    pub acceptance_rate: f32,
    pub iterations: u32,
    pub content_length: usize,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of speculative decoding failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SpeculativeFaultType {
    DraftFailed,
    VerificationFailed,
    MaxIterationsExceeded,
}

/// Emitted when a speculative decoding operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct SpeculativeFault {
    pub error_type: SpeculativeFaultType,
    pub message: String,
}

impl SpeculativeFault {
    pub fn draft_failed(msg: impl Into<String>) -> Self {
        Self {
            error_type: SpeculativeFaultType::DraftFailed,
            message: msg.into(),
        }
    }

    pub fn verification_failed(msg: impl Into<String>) -> Self {
        Self {
            error_type: SpeculativeFaultType::VerificationFailed,
            message: msg.into(),
        }
    }

    pub fn max_iterations_exceeded(msg: impl Into<String>) -> Self {
        Self {
            error_type: SpeculativeFaultType::MaxIterationsExceeded,
            message: msg.into(),
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks the current state of speculative decoding.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct SpeculativeState {
    pub total_runs: u64,
    pub total_draft_tokens: u64,
    pub total_accepted_tokens: u64,
    pub avg_acceptance_rate: f32,
}
