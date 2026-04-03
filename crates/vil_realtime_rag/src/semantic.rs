//! Semantic types for real-time RAG operations.
//!
//! VIL process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after a real-time RAG query completes.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct RealtimeRagEvent {
    pub query_embedding_dim: usize,
    pub chunks_retrieved: u32,
    pub from_cache: bool,
    pub search_time_ns: u64,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of real-time RAG failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RealtimeRagFaultType {
    IndexEmpty,
    DimensionMismatch,
    CacheFull,
}

/// Emitted when a real-time RAG operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct RealtimeRagFault {
    pub error_type: RealtimeRagFaultType,
    pub message: String,
}

impl RealtimeRagFault {
    pub fn index_empty(msg: impl Into<String>) -> Self {
        Self {
            error_type: RealtimeRagFaultType::IndexEmpty,
            message: msg.into(),
        }
    }

    pub fn dimension_mismatch(msg: impl Into<String>) -> Self {
        Self {
            error_type: RealtimeRagFaultType::DimensionMismatch,
            message: msg.into(),
        }
    }

    pub fn cache_full(msg: impl Into<String>) -> Self {
        Self {
            error_type: RealtimeRagFaultType::CacheFull,
            message: msg.into(),
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks the current state of a real-time RAG pipeline.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct RealtimeRagState {
    pub doc_count: u64,
    pub cache_entries: u64,
    pub total_queries: u64,
    pub avg_search_time_ns: u64,
}
