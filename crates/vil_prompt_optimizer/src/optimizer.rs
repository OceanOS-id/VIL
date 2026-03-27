//! PromptOptimizer — evaluate candidates, find the best, generate variations.

use crate::candidate::{EvaluationResult, PromptCandidate};
use crate::evaluator::{PromptEvaluator, TestCase};
use crate::strategy::OptimizeStrategy;
use std::sync::Arc;

/// Prompt optimizer that manages candidates, evaluates them, and suggests variations.
pub struct PromptOptimizer {
    pub candidates: Vec<PromptCandidate>,
    pub evaluator: Arc<dyn PromptEvaluator>,
    pub strategy: OptimizeStrategy,
}

impl PromptOptimizer {
    pub fn new(evaluator: Arc<dyn PromptEvaluator>, strategy: OptimizeStrategy) -> Self {
        Self {
            candidates: Vec::new(),
            evaluator,
            strategy,
        }
    }

    /// Add a candidate prompt template.
    pub fn add_candidate(&mut self, template: &str) {
        self.candidates.push(PromptCandidate::new(template));
    }

    /// Evaluate all candidates against test cases, returning (template, avg_score) pairs.
    pub async fn evaluate_all(&mut self, test_cases: &[TestCase]) -> Vec<(String, f32)> {
        let indices: Vec<usize> = match self.strategy {
            OptimizeStrategy::GridSearch => (0..self.candidates.len()).collect(),
            OptimizeStrategy::RandomSearch => {
                // Simple deterministic "random" subset: every other candidate.
                (0..self.candidates.len()).step_by(2).collect()
            }
            OptimizeStrategy::Bayesian => {
                // Simplified: evaluate all, but could prioritize top performers.
                (0..self.candidates.len()).collect()
            }
        };

        let mut results = Vec::new();

        for &idx in &indices {
            let template = self.candidates[idx].template.clone();
            let mut evals = Vec::new();

            for tc in test_cases {
                let score = self.evaluator.evaluate(&template, tc).await;
                evals.push(EvaluationResult {
                    test_case: tc.input.clone(),
                    score,
                    notes: None,
                });
            }

            let avg = if evals.is_empty() {
                0.0
            } else {
                evals.iter().map(|e| e.score).sum::<f32>() / evals.len() as f32
            };

            self.candidates[idx].evaluations = evals;
            self.candidates[idx].score = avg;
            results.push((template, avg));
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Return the highest scoring candidate.
    pub fn best(&self) -> Option<&PromptCandidate> {
        self.candidates
            .iter()
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Generate a variation of a prompt template via simple mutations.
    pub fn suggest_variation(&self, template: &str) -> String {
        let mutations = [
            ("", " Let's think step by step."),
            ("", " Be concise."),
            ("", " Provide a detailed answer."),
            ("", " Answer in JSON format."),
            ("You are", "You are an expert"),
        ];

        // Pick a mutation based on template length (deterministic).
        let idx = template.len() % mutations.len();
        let (find, replace_or_append) = mutations[idx];

        if find.is_empty() {
            // Append mutation.
            format!("{template}{replace_or_append}")
        } else if template.contains(find) {
            template.replacen(find, replace_or_append, 1)
        } else {
            // Fallback: append.
            format!("{template} Please be precise.")
        }
    }
}
