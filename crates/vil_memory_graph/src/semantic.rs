//! Semantic types for memory graph operations.
//!
//! VIL process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after a memory graph operation completes.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct MemoryEvent {
    pub operation: String,
    pub entity_count: u64,
    pub relation_count: u64,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of memory graph failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MemoryFaultType {
    EntityNotFound,
    RelationCycle,
    StorageCorrupted,
}

/// Emitted when a memory graph operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct MemoryFault {
    pub error_type: MemoryFaultType,
    pub message: String,
}

impl MemoryFault {
    pub fn entity_not_found(msg: impl Into<String>) -> Self {
        Self { error_type: MemoryFaultType::EntityNotFound, message: msg.into() }
    }

    pub fn relation_cycle(msg: impl Into<String>) -> Self {
        Self { error_type: MemoryFaultType::RelationCycle, message: msg.into() }
    }

    pub fn storage_corrupted(msg: impl Into<String>) -> Self {
        Self { error_type: MemoryFaultType::StorageCorrupted, message: msg.into() }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks the current state of the memory graph.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct MemoryState {
    pub entity_count: u64,
    pub relation_count: u64,
    pub total_recalls: u64,
    pub avg_recall_results: f32,
}
