//! Prompt candidates for optimization.

use serde::{Deserialize, Serialize};

/// A prompt candidate with its template, score, and evaluation history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptCandidate {
    pub template: String,
    pub score: f32,
    pub evaluations: Vec<EvaluationResult>,
}

/// Result of evaluating a candidate against a test case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub test_case: String,
    pub score: f32,
    pub notes: Option<String>,
}

impl PromptCandidate {
    pub fn new(template: &str) -> Self {
        Self {
            template: template.to_string(),
            score: 0.0,
            evaluations: Vec::new(),
        }
    }

    /// Compute average score from evaluations.
    pub fn avg_score(&self) -> f32 {
        if self.evaluations.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.evaluations.iter().map(|e| e.score).sum();
        sum / self.evaluations.len() as f32
    }
}
