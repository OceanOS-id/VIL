//! Evaluator trait for evaluation metrics.

use serde::{Deserialize, Serialize};

/// Score produced by an evaluation metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricScore {
    /// Name of the metric.
    pub name: String,
    /// Score in [0.0, 1.0].
    pub score: f32,
    /// Additional details as JSON.
    pub details: serde_json::Value,
}

/// Trait for evaluation metrics.
pub trait EvalMetric: Send + Sync {
    /// Evaluate the quality of an answer given question, answer, context, and optional reference.
    fn evaluate(
        &self,
        question: &str,
        answer: &str,
        context: &str,
        reference: Option<&str>,
    ) -> MetricScore;
}
