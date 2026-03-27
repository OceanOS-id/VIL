// =============================================================================
// VIL Semantic Types — Streaming RAG
// =============================================================================

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

/// Events emitted by the Streaming RAG subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiEvent)]
pub enum StreamingRagEvent {
    /// A text chunk was ingested and indexed.
    ChunkIngested { chunk_index: usize, text_len: usize },
    /// A buffer flush was triggered.
    BufferFlushed { chunks_produced: usize },
    /// A search query was executed.
    SearchExecuted { top_k: usize, results_found: usize },
    /// The configuration was updated.
    ConfigUpdated { chunk_size: usize, overlap: usize },
}

/// Faults that can occur in the Streaming RAG subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiFault)]
pub enum StreamingRagFault {
    /// The ingestion buffer overflowed.
    BufferOverflow { buffer_len: usize, max_size: usize },
    /// The embedding computation failed.
    EmbeddingFailed { reason: String },
    /// The index write failed.
    IndexWriteFailed { reason: String },
}

/// Observable state of the Streaming RAG subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiState)]
pub struct StreamingRagState {
    /// Number of chunks currently indexed.
    pub chunk_count: usize,
    /// Current buffer length in characters.
    pub buffer_len: usize,
    /// Active chunk size configuration.
    pub chunk_size: usize,
    /// Active overlap configuration.
    pub overlap: usize,
}
