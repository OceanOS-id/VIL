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

pub mod strategy;
pub mod semantic;
pub mod sliding;
pub mod code;
pub mod table;

pub use strategy::{ChunkStrategy, ChunkType, ChunkMeta, TextChunk, estimate_tokens};
pub use semantic::SentenceChunker;
pub use sliding::SlidingWindowChunker;
pub use code::CodeChunker;
pub use table::TableChunker;

// VIL integration layer
pub mod vil_semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use plugin::ChunkerPlugin;
pub use vil_semantic::{ChunkEvent, ChunkFault, ChunkerState};
