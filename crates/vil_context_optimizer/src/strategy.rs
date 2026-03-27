//! Optimization strategies for context window compression.

use serde::{Deserialize, Serialize};

/// Strategy for optimizing context chunks to fit a token budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizeStrategy {
    /// Keep top-K chunks by importance score.
    TopK(usize),
    /// Fit as many chunks as possible within token budget (by score order).
    BudgetFit,
    /// Deduplicate then budget-fit.
    DedupAndFit {
        /// Jaccard threshold above which chunks are considered duplicates.
        dedup_threshold: f32,
    },
    /// Full pipeline: dedup + score + budget-fit.
    Full {
        /// Jaccard threshold above which chunks are considered duplicates.
        dedup_threshold: f32,
    },
}

impl Default for OptimizeStrategy {
    fn default() -> Self {
        OptimizeStrategy::Full {
            dedup_threshold: 0.8,
        }
    }
}
