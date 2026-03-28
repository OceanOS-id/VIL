//! # vil_realtime_rag
//!
//! Sub-millisecond RAG pipeline for latency-critical applications
//! (trading, gaming, robotics).
//!
//! ## Key difference from `vil_rag`
//!
//! | Aspect | `vil_rag` | `vil_realtime_rag` |
//! |--------|-------------|----------------------|
//! | Embedding | API-based (network latency) | Pre-computed, local |
//! | Index | Disk-backed / hybrid | In-memory, contiguous |
//! | Target latency | 100ms+ | <1ms total |
//!
//! ## Quick start
//!
//! ```rust
//! use vil_realtime_rag::{RealtimeRagPipeline, RealtimeRagConfig};
//!
//! let pipeline = RealtimeRagPipeline::new(RealtimeRagConfig {
//!     dimension: 4,
//!     top_k: 3,
//!     ..Default::default()
//! });
//!
//! // Add documents with pre-computed embeddings.
//! pipeline.add_document(&[1.0, 0.0, 0.0, 0.0], "d1", "hello world", serde_json::json!({}));
//! pipeline.add_document(&[0.0, 1.0, 0.0, 0.0], "d2", "goodbye world", serde_json::json!({}));
//!
//! // Query with a pre-computed embedding (<1ms path).
//! let result = pipeline.query_with_embedding(&[1.0, 0.0, 0.0, 0.0]);
//! assert_eq!(result.chunks[0].doc_id, "d1");
//! ```

pub mod bench;
pub mod config;
pub mod index;
pub mod pipeline;
pub mod query_cache;

// Re-exports for convenience.
pub use bench::{bench_search, BenchResult};
pub use config::RealtimeRagConfig;
pub use index::{DocEntry, RealtimeIndex, RealtimeResult};
pub use pipeline::{RealtimeQueryResult, RealtimeRagPipeline};
pub use query_cache::QueryCache;

pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod semantic;

pub use plugin::RealtimeRagPlugin;
pub use semantic::{RealtimeRagEvent, RealtimeRagFault, RealtimeRagState};
