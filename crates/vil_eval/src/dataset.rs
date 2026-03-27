//! EvalDataset — test cases for evaluation.

use serde::{Deserialize, Serialize};

/// A single evaluation test case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalCase {
    /// The question / input prompt.
    pub question: String,
    /// The context provided to the LLM.
    pub context: String,
    /// The answer produced by the LLM.
    pub answer: String,
    /// Optional reference (gold-standard) answer.
    pub reference: Option<String>,
}

/// A dataset of evaluation cases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalDataset {
    /// The test cases.
    pub cases: Vec<EvalCase>,
}

impl EvalDataset {
    /// Create an empty dataset.
    pub fn new() -> Self {
        Self { cases: Vec::new() }
    }

    /// Add a case to the dataset.
    pub fn add_case(&mut self, case: EvalCase) {
        self.cases.push(case);
    }

    /// Load dataset from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize dataset to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Number of cases in the dataset.
    pub fn len(&self) -> usize {
        self.cases.len()
    }

    /// Check if dataset is empty.
    pub fn is_empty(&self) -> bool {
        self.cases.is_empty()
    }
}

impl Default for EvalDataset {
    fn default() -> Self {
        Self::new()
    }
}
