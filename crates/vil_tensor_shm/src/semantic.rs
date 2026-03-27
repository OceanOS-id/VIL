//! Semantic types for tensor SHM operations.
//!
//! VIL process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after a tensor allocation or write to the pool.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct TensorAllocEvent {
    pub buffer_index: usize,
    pub shape: Vec<usize>,
    pub dtype: String,
    pub byte_size: usize,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of tensor SHM failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TensorFaultType {
    BufferFull,
    ShapeMismatch,
    InvalidDescriptor,
}

/// Emitted when a tensor SHM operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct TensorFault {
    pub error_type: TensorFaultType,
    pub message: String,
}

impl TensorFault {
    pub fn buffer_full(msg: impl Into<String>) -> Self {
        Self { error_type: TensorFaultType::BufferFull, message: msg.into() }
    }

    pub fn shape_mismatch(msg: impl Into<String>) -> Self {
        Self { error_type: TensorFaultType::ShapeMismatch, message: msg.into() }
    }

    pub fn invalid_descriptor(msg: impl Into<String>) -> Self {
        Self { error_type: TensorFaultType::InvalidDescriptor, message: msg.into() }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks the current state of a tensor pool.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct TensorPoolState {
    pub buffer_count: u64,
    pub total_allocs: u64,
    pub total_bytes_written: u64,
    pub active_descriptors: u64,
}
