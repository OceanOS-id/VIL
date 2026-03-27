//! Evaluation interface for prompt candidates.

use async_trait::async_trait;

/// A test case for evaluating prompts.
#[derive(Debug, Clone)]
pub struct TestCase {
    pub input: String,
    pub expected_output: String,
}

/// Trait for evaluating a prompt against test cases.
#[async_trait]
pub trait PromptEvaluator: Send + Sync {
    /// Evaluate a prompt template against a single test case.
    /// Returns a score between 0.0 and 1.0.
    async fn evaluate(&self, template: &str, test_case: &TestCase) -> f32;
}

/// A simple evaluator that scores based on keyword overlap with expected output.
pub struct KeywordOverlapEvaluator;

#[async_trait]
impl PromptEvaluator for KeywordOverlapEvaluator {
    async fn evaluate(&self, template: &str, test_case: &TestCase) -> f32 {
        // Simple heuristic: does the template incorporate keywords from the expected output?
        let template_lower = template.to_lowercase();
        let expected_words: Vec<&str> = test_case.expected_output.split_whitespace().collect();

        if expected_words.is_empty() {
            return 0.5;
        }

        let matches = expected_words
            .iter()
            .filter(|w| template_lower.contains(&w.to_lowercase()))
            .count();

        (matches as f32 / expected_words.len() as f32).min(1.0)
    }
}
