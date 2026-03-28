//! VIL Advanced Semantic Chunker (H01)
//!
//! Extends the base `vil_rag` chunker with high-performance, strategy-based
//! text chunking optimised for RAG pipelines.
//!
//! ## Strategies
//!
//! | Strategy | Description |
//! |---|---|
//! | [`SentenceChunker`] | Sentence-boundary splitting with token-budget merging |
//! | [`SlidingWindowChunker`] | Fixed word-window with configurable overlap |
//! | [`CodeChunker`] | Function / class boundary detection |
//! | [`TableChunker`] | CSV / table row batching with header preservation |
//!
//! All strategies implement the [`ChunkStrategy`] trait.

pub mod code;
pub mod semantic;
pub mod sliding;
pub mod strategy;
pub mod table;

pub use code::CodeChunker;
pub use semantic::SentenceChunker;
pub use sliding::SlidingWindowChunker;
pub use strategy::{estimate_tokens, ChunkMeta, ChunkStrategy, ChunkType, TextChunk};
pub use table::TableChunker;

// VIL integration layer
pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod vil_semantic;

pub use plugin::ChunkerPlugin;
pub use vil_semantic::{ChunkEvent, ChunkFault, ChunkerState};
