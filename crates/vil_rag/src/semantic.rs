//! Semantic types for RAG operations.
//!
//! These types follow VIL's process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)
//!
//! Each type uses Tier B AI semantic derive macros so the VIL runtime
//! can route them to the correct tri-lane automatically.

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after a RAG query completes (retrieval + generation).
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct RagQueryEvent {
    pub question: String,
    pub chunks_retrieved: u32,
    pub answer_length: u32,
    pub latency_ms: u64,
    pub model: String,
}

/// Emitted after a document ingestion completes.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct RagIngestEvent {
    pub doc_id: String,
    pub chunks_created: u32,
    pub embeddings_generated: u32,
    pub latency_ms: u64,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of RAG failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RagFaultType {
    EmbeddingFailed,
    StoreFailed,
    RetrievalFailed,
    GenerationFailed,
}

/// Emitted when a RAG operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct RagFault {
    pub error_type: RagFaultType,
    pub message: String,
}

impl RagFault {
    /// Convenience constructor for embedding failures.
    pub fn embedding_failed(msg: impl Into<String>) -> Self {
        Self {
            error_type: RagFaultType::EmbeddingFailed,
            message: msg.into(),
        }
    }

    /// Convenience constructor for store failures.
    pub fn store_failed(msg: impl Into<String>) -> Self {
        Self {
            error_type: RagFaultType::StoreFailed,
            message: msg.into(),
        }
    }

    /// Convenience constructor for retrieval failures.
    pub fn retrieval_failed(msg: impl Into<String>) -> Self {
        Self {
            error_type: RagFaultType::RetrievalFailed,
            message: msg.into(),
        }
    }

    /// Convenience constructor for generation failures.
    pub fn generation_failed(msg: impl Into<String>) -> Self {
        Self {
            error_type: RagFaultType::GenerationFailed,
            message: msg.into(),
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks the current state of a RAG index.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct RagIndexState {
    pub doc_count: u64,
    pub chunk_count: u64,
    pub total_embeddings: u64,
    /// Unix timestamp of the last successful ingestion.
    pub last_indexed_at: u64,
    pub store_type: String,
}

impl RagIndexState {
    /// Update state after a successful ingestion.
    pub fn record_ingest(&mut self, event: &RagIngestEvent, now_unix: u64) {
        self.doc_count += 1;
        self.chunk_count += event.chunks_created as u64;
        self.total_embeddings += event.embeddings_generated as u64;
        self.last_indexed_at = now_unix;
    }
}
