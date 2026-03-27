use crate::metrics::VariantMetrics;
use serde::{Serialize, Deserialize};

/// Promotion / rollback policy for model variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PromotionPolicy {
    /// Manually promote/rollback only.
    Manual,
    /// Automatically promote a variant when it meets the criteria.
    AutoPromote {
        /// Minimum number of requests before auto-promote can trigger.
        min_requests: u64,
        /// Minimum average quality score required.
        min_quality: f64,
    },
    /// Automatically roll back a variant when it exceeds error thresholds.
    AutoRollback {
        /// Maximum error rate (0.0 – 1.0) before rollback triggers.
        max_error_rate: f64,
        /// Minimum requests before evaluation.
        min_requests: u64,
    },
}

impl Default for PromotionPolicy {
    fn default() -> Self {
        Self::Manual
    }
}

impl PromotionPolicy {
    /// Evaluate whether a variant should be promoted based on its metrics.
    pub fn should_promote(&self, metrics: &VariantMetrics) -> bool {
        match self {
            Self::AutoPromote { min_requests, min_quality } => {
                metrics.requests >= *min_requests && metrics.avg_quality_score >= *min_quality
            }
            _ => false,
        }
    }

    /// Evaluate whether a variant should be rolled back based on its metrics.
    pub fn should_rollback(&self, metrics: &VariantMetrics) -> bool {
        match self {
            Self::AutoRollback { max_error_rate, min_requests } => {
                metrics.requests >= *min_requests && metrics.error_rate() > *max_error_rate
            }
            _ => false,
        }
    }
}
