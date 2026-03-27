//! Optimization strategies for prompt search.

use serde::{Deserialize, Serialize};

/// Strategy for exploring the prompt space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizeStrategy {
    /// Evaluate all candidates.
    GridSearch,
    /// Randomly sample a subset of candidates.
    RandomSearch,
    /// Simplified Bayesian: prioritize candidates similar to top performers.
    Bayesian,
}

impl Default for OptimizeStrategy {
    fn default() -> Self {
        Self::GridSearch
    }
}

impl std::fmt::Display for OptimizeStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GridSearch => write!(f, "GridSearch"),
            Self::RandomSearch => write!(f, "RandomSearch"),
            Self::Bayesian => write!(f, "Bayesian"),
        }
    }
}
