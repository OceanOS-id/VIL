//! EvalReport — evaluation results and summary.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use vil_macros::VilAiEvent;

use crate::evaluator::MetricScore;

/// Per-case evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseResult {
    /// Index of the case in the dataset.
    pub case_index: usize,
    /// Scores from each metric.
    pub scores: Vec<MetricScore>,
}

/// Aggregate evaluation report.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiEvent)]
pub struct EvalReport {
    /// Per-case results.
    pub results: Vec<CaseResult>,
    /// Summary: metric name -> average score.
    pub summary: HashMap<String, f32>,
}

impl EvalReport {
    /// Create an empty report.
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            summary: HashMap::new(),
        }
    }

    /// Compute summary (averages) from case results.
    pub fn compute_summary(&mut self) {
        let mut totals: HashMap<String, (f32, usize)> = HashMap::new();

        for case_result in &self.results {
            for score in &case_result.scores {
                let entry = totals.entry(score.name.clone()).or_insert((0.0, 0));
                entry.0 += score.score;
                entry.1 += 1;
            }
        }

        self.summary = totals
            .into_iter()
            .map(|(name, (total, count))| (name, total / count as f32))
            .collect();
    }

    /// Number of cases evaluated.
    pub fn case_count(&self) -> usize {
        self.results.len()
    }

    /// Get summary score for a specific metric.
    pub fn metric_average(&self, name: &str) -> Option<f32> {
        self.summary.get(name).copied()
    }
}

impl Default for EvalReport {
    fn default() -> Self {
        Self::new()
    }
}
