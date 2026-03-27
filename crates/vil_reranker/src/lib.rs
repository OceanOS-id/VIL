//! VIL Reranker Engine (H03)
//!
//! Provides reranking strategies for RAG retrieval pipelines:
//!
//! | Strategy | Description |
//! |---|---|
//! | [`KeywordReranker`] | Boosts candidates containing query keywords |
//! | [`CrossEncoderReranker`] | Cosine-similarity scoring via embedder |
//! | [`RRFReranker`] | Reciprocal Rank Fusion for combining multiple ranked lists |
//!
//! All strategies implement the async [`Reranker`] trait.

pub mod reranker;
pub mod keyword;
pub mod cross_encoder;
pub mod fusion;

pub use reranker::{Reranker, RerankCandidate, RerankResult, RerankError};
pub use keyword::KeywordReranker;
pub use cross_encoder::CrossEncoderReranker;
pub use fusion::RRFReranker;

// VIL integration layer
pub mod semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use plugin::RerankerPlugin;
pub use semantic::{RerankEvent, RerankFault, RerankerState};
