// ── N04: Benchmark Trait ────────────────────────────────────────────
use serde::{Deserialize, Serialize};

/// A single benchmark test case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchCase {
    pub question: String,
    pub expected_answer: String,
    pub category: String,
}

impl BenchCase {
    pub fn new(
        question: impl Into<String>,
        expected_answer: impl Into<String>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            question: question.into(),
            expected_answer: expected_answer.into(),
            category: category.into(),
        }
    }
}

/// Core benchmark trait — implement to add custom LLM benchmarks.
pub trait Benchmark: Send + Sync {
    /// Human-readable benchmark name.
    fn name(&self) -> &str;

    /// Return all test cases for this benchmark.
    fn cases(&self) -> Vec<BenchCase>;

    /// Evaluate a model's answer against the expected answer.
    /// Returns a score in 0.0–1.0.
    fn evaluate(&self, answer: &str, expected: &str) -> f32;
}
