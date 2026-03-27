//! VIL Context Optimizer
//!
//! Intelligently compresses and optimizes LLM context windows to fit
//! more information in fewer tokens. Supports deduplication, importance
//! scoring, and budget-aware chunk selection.
//!
//! # Example
//!
//! ```rust,no_run
//! use vil_context_optimizer::{
//!     ContextOptimizer, TokenBudget, OptimizeStrategy,
//! };
//!
//! let budget = TokenBudget::new(8000)
//!     .system_tokens(500)
//!     .response_tokens(1000);
//!
//! let optimizer = ContextOptimizer::new(budget)
//!     .strategy(OptimizeStrategy::Full { dedup_threshold: 0.8 });
//!
//! let chunks = vec![
//!     ("Rust is a systems language.".into(), 0.9),
//!     ("Rust is a systems language.".into(), 0.9),  // duplicate
//!     ("Python is great for ML.".into(), 0.7),
//! ];
//!
//! let result = optimizer.optimize(&chunks);
//! println!("Kept {}/{} chunks, saved {} tokens",
//!     result.final_count, result.original_count, result.tokens_saved);
//! ```

pub mod budget;
pub mod dedup;
pub mod optimizer;
pub mod scorer;
pub mod strategy;
pub mod semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use budget::TokenBudget;
pub use dedup::deduplicate;
pub use optimizer::{ContextOptimizer, OptimizedContext};
pub use scorer::{score_chunks, ChunkScore, ScoringWeights};
pub use strategy::OptimizeStrategy;
pub use plugin::OptimizerPlugin;
pub use semantic::{OptimizeEvent, OptimizeFault, OptimizeFaultType, OptimizerState};
