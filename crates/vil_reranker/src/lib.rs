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

pub mod cross_encoder;
pub mod fusion;
pub mod keyword;
pub mod reranker;

pub use cross_encoder::CrossEncoderReranker;
pub use fusion::RRFReranker;
pub use keyword::KeywordReranker;
pub use reranker::{RerankCandidate, RerankError, RerankResult, Reranker};

// VIL integration layer
pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod semantic;

pub use plugin::RerankerPlugin;
pub use semantic::{RerankEvent, RerankFault, RerankerState};
