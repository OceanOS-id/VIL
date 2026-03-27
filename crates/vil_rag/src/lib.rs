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
pub mod pipeline;
pub mod retriever;
pub mod store;
pub mod semantic;
pub mod extractors;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use chunk::{Chunk, EmbeddedChunk, ChunkerStrategy, FixedChunker, SemanticChunker, MarkdownChunker};
pub use config::{ChunkerType, StoreType};
pub use document::{DocumentParser, PlainTextParser, MarkdownParser};
pub use pipeline::{RagPipeline, RagPipelineBuilder, RagError, IngestResult, QueryResult};
pub use retriever::{Retriever, DenseRetriever, RetrievedChunk};
pub use store::{VectorStore, InMemoryStore};
pub use plugin::RagPlugin;
pub use extractors::Rag;
pub use semantic::{RagQueryEvent, RagIngestEvent, RagFault, RagFaultType, RagIndexState};
