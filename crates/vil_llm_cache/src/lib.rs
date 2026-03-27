//! # vil_llm_cache (H05)
//!
//! Semantic Response Cache for VIL.
//!
//! Provides both exact-match (FNV hash) and embedding-based similarity caching
//! for LLM responses. Reduces redundant API calls by returning cached responses
//! when semantically equivalent queries are detected.
//!
//! ## Quick start
//!
//! ```rust
//! use vil_llm_cache::{SemanticCache, CacheConfig};
//!
//! let cache = SemanticCache::new(CacheConfig::default());
//!
//! // Store a response
//! cache.put("What is Rust?", Some(vec![0.1, 0.9, 0.3]), "Rust is a systems language.".into(), "gpt-4".into());
//!
//! // Exact match
//! let hit = cache.get_exact("What is Rust?");
//! assert!(hit.is_some());
//! ```

pub mod cache;
pub mod config;
pub mod hasher;
pub mod similarity;

// Re-exports
pub use cache::{CachedResponse, CacheStats, SemanticCache};
pub use config::CacheConfig;
pub use hasher::{fnv1a_hash, hash_messages};

// VIL integration layer
pub mod semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use plugin::LlmCachePlugin;
pub use semantic::{CacheHitEvent, CacheFault, LlmCacheState};
