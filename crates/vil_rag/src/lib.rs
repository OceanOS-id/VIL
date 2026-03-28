//! VIL RAG Plugin — Retrieval-Augmented Generation pipeline.
//!
//! Provides document ingestion (chunk -> embed -> store) and query
//! (retrieve -> generate) via a VIL plugin registered through
//! `VilApp::new("app").plugin(RagPlugin::new())`.
//!
//! Depends on `vil_llm` for LlmProvider and EmbeddingProvider.
//!
//! # Plugin endpoints
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | POST | /api/rag/ingest | Ingest a document |
//! | POST | /api/rag/query | RAG query |
//! | GET | /api/rag/stats | Index stats |

pub mod chunk;
pub mod config;
pub mod document;
pub mod extractors;
pub mod handlers;
pub mod pipeline;
pub mod pipeline_sse;
pub mod plugin;
pub mod retriever;
pub mod semantic;
pub mod store;

pub use chunk::{
    Chunk, ChunkerStrategy, EmbeddedChunk, FixedChunker, MarkdownChunker, SemanticChunker,
};
pub use config::{ChunkerType, StoreType};
pub use document::{DocumentParser, MarkdownParser, PlainTextParser};
pub use extractors::Rag;
pub use pipeline::{IngestResult, QueryResult, RagError, RagPipeline, RagPipelineBuilder};
pub use plugin::RagPlugin;
pub use retriever::{DenseRetriever, RetrievedChunk, Retriever};
pub use semantic::{RagFault, RagFaultType, RagIndexState, RagIngestEvent, RagQueryEvent};
pub use store::{InMemoryStore, VectorStore};
