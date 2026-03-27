// ── N03: Preference Pair ────────────────────────────────────────────
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single RLHF preference pair: chosen > rejected for a given prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferencePair {
    pub prompt: String,
    pub chosen: String,
    pub rejected: String,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl PreferencePair {
    pub fn new(prompt: impl Into<String>, chosen: impl Into<String>, rejected: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            chosen: chosen.into(),
            rejected: rejected.into(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Chosen response length.
    pub fn chosen_length(&self) -> usize {
        self.chosen.len()
    }

    /// Rejected response length.
    pub fn rejected_length(&self) -> usize {
        self.rejected.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_preference_pair() {
        let pair = PreferencePair::new("What is 2+2?", "4", "5");
        assert_eq!(pair.prompt, "What is 2+2?");
        assert_eq!(pair.chosen, "4");
        assert_eq!(pair.rejected, "5");
        assert!(pair.metadata.is_empty());
    }

    #[test]
    fn preference_pair_with_metadata() {
        let pair = PreferencePair::new("prompt", "good", "bad")
            .with_metadata("annotator", "human_01")
            .with_metadata("confidence", "high");
        assert_eq!(pair.metadata.len(), 2);
        assert_eq!(pair.metadata["annotator"], "human_01");
    }
}
