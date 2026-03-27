use serde::{Deserialize, Serialize};

/// Chunking strategy type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkerType {
    /// Fixed-size chunking with overlap.
    Fixed { chunk_size: usize, overlap: usize },
    /// Semantic chunking — splits on sentence boundaries, merges until chunk_size.
    Semantic { chunk_size: usize, overlap: usize },
    /// Markdown-aware chunking — splits on headers.
    Markdown { chunk_size: usize },
}

impl Default for ChunkerType {
    fn default() -> Self {
        Self::Semantic {
            chunk_size: 512,
            overlap: 50,
        }
    }
}

/// Vector store backend type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StoreType {
    /// In-memory brute-force cosine similarity (dev/testing).
    InMemory,
    // Future: Qdrant { url: String, collection: String },
}

impl Default for StoreType {
    fn default() -> Self {
        Self::InMemory
    }
}

/// RAG plugin configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    pub chunker: ChunkerType,
    pub store: StoreType,
    pub top_k: usize,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            chunker: ChunkerType::default(),
            store: StoreType::default(),
            top_k: 5,
        }
    }
}
