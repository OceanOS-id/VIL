//! # vil_vectordb
//!
//! Native Rust vector database with HNSW (Hierarchical Navigable Small World) index,
//! designed for single-binary RAG pipelines — no external database required.
//!
//! ## Quick Start
//!
//! ```rust
//! use vil_vectordb::{Collection, HnswConfig, QueryBuilder};
//!
//! let col = Collection::new("docs", 3, HnswConfig::default());
//! col.add(vec![1.0, 0.0, 0.0], serde_json::json!({"title": "doc1"}), Some("hello".into()));
//! col.add(vec![0.0, 1.0, 0.0], serde_json::json!({"title": "doc2"}), None);
//!
//! let results = col.search(&[1.0, 0.0, 0.0], 5);
//! assert!(!results.is_empty());
//!
//! let results = QueryBuilder::new(&col)
//!     .vector(vec![1.0, 0.0, 0.0])
//!     .top_k(5)
//!     .min_score(0.5)
//!     .execute();
//! ```

pub mod collection;
pub mod config;
pub mod distance;
pub mod hnsw;
pub mod query;
pub mod storage;

// Re-exports for convenience
pub use collection::{Collection, SearchResult};
pub use config::HnswConfig;
pub use distance::DistanceMetric;
pub use hnsw::{HnswIndex, SearchHit, VectorDbError};
pub use query::QueryBuilder;
pub use storage::{VectorRecord, VectorStorage};

pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod semantic;

pub use plugin::VectorDbPlugin;
pub use semantic::{IndexEvent, SearchEvent, VectorDbFault, VectorDbState};
