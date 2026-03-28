//! # vil_embedder
//!
//! High-performance text embedding engine for VIL.
//!
//! Supports API-based providers (OpenAI) and local model inference (ONNX, planned).
//! Features concurrent batch processing and SIMD-friendly vector similarity functions.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use vil_embedder::{OpenAiEmbedder, BatchEmbedder, EmbedProvider};
//! use vil_embedder::similarity::cosine_similarity;
//!
//! # async fn example() {
//! let provider = Arc::new(OpenAiEmbedder::new("sk-..."));
//! let batcher = BatchEmbedder::new(provider.clone());
//!
//! let texts = vec!["hello world".to_string(), "goodbye world".to_string()];
//! let embeddings = batcher.embed_all(&texts).await.unwrap();
//!
//! let sim = cosine_similarity(&embeddings[0], &embeddings[1]);
//! println!("similarity: {sim}");
//! # }
//! ```

pub mod batch;
pub mod normalize;
pub mod openai;
pub mod provider;
pub mod similarity;

// Re-exports for convenience.
pub use batch::BatchEmbedder;
pub use normalize::{l2_normalize, l2_normalize_batch};
pub use openai::OpenAiEmbedder;
pub use provider::{EmbedError, EmbedProvider};

pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod semantic;

pub use plugin::EmbedderPlugin;
pub use semantic::{EmbedEvent, EmbedFault, EmbedderState};
