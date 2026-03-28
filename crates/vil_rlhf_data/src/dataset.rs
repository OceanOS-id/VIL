// ── N03: Preference Dataset ─────────────────────────────────────────
use crate::preference::PreferencePair;
use serde::{Deserialize, Serialize};

/// Aggregate statistics for a preference dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetStats {
    pub total_pairs: usize,
    pub avg_prompt_length: f64,
    pub avg_chosen_length: f64,
    pub avg_rejected_length: f64,
}

/// A collection of preference pairs for RLHF/DPO training.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PreferenceDataset {
    pub pairs: Vec<PreferencePair>,
}

impl PreferenceDataset {
    pub fn new() -> Self {
        Self { pairs: Vec::new() }
    }

    /// Add a preference pair.
    pub fn add_pair(
        &mut self,
        prompt: impl Into<String>,
        chosen: impl Into<String>,
        rejected: impl Into<String>,
    ) {
        self.pairs
            .push(PreferencePair::new(prompt, chosen, rejected));
    }

    /// Add a pre-constructed pair.
    pub fn push(&mut self, pair: PreferencePair) {
        self.pairs.push(pair);
    }

    /// Number of pairs.
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// Remove a pair by index.
    pub fn remove(&mut self, index: usize) -> Option<PreferencePair> {
        if index < self.pairs.len() {
            Some(self.pairs.remove(index))
        } else {
            None
        }
    }

    /// Compute dataset statistics.
    pub fn stats(&self) -> DatasetStats {
        if self.pairs.is_empty() {
            return DatasetStats {
                total_pairs: 0,
                avg_prompt_length: 0.0,
                avg_chosen_length: 0.0,
                avg_rejected_length: 0.0,
            };
        }

        let n = self.pairs.len() as f64;
        let avg_prompt = self.pairs.iter().map(|p| p.prompt.len()).sum::<usize>() as f64 / n;
        let avg_chosen = self.pairs.iter().map(|p| p.chosen.len()).sum::<usize>() as f64 / n;
        let avg_rejected = self.pairs.iter().map(|p| p.rejected.len()).sum::<usize>() as f64 / n;

        DatasetStats {
            total_pairs: self.pairs.len(),
            avg_prompt_length: avg_prompt,
            avg_chosen_length: avg_chosen,
            avg_rejected_length: avg_rejected,
        }
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dataset_add_and_len() {
        let mut ds = PreferenceDataset::new();
        ds.add_pair("p1", "good1", "bad1");
        ds.add_pair("p2", "good2", "bad2");
        assert_eq!(ds.len(), 2);
    }

    #[test]
    fn dataset_remove() {
        let mut ds = PreferenceDataset::new();
        ds.add_pair("p1", "good1", "bad1");
        ds.add_pair("p2", "good2", "bad2");
        let removed = ds.remove(0);
        assert!(removed.is_some());
        assert_eq!(ds.len(), 1);
        assert_eq!(ds.pairs[0].prompt, "p2");
    }

    #[test]
    fn dataset_stats() {
        let mut ds = PreferenceDataset::new();
        ds.add_pair("hello", "good answer", "bad");
        ds.add_pair("world", "another good", "nope");
        let stats = ds.stats();
        assert_eq!(stats.total_pairs, 2);
        assert!(stats.avg_prompt_length > 0.0);
    }

    #[test]
    fn dataset_stats_empty() {
        let ds = PreferenceDataset::new();
        let stats = ds.stats();
        assert_eq!(stats.total_pairs, 0);
        assert_eq!(stats.avg_prompt_length, 0.0);
    }

    #[test]
    fn dataset_json_roundtrip() {
        let mut ds = PreferenceDataset::new();
        ds.add_pair("prompt", "chosen", "rejected");
        let json = ds.to_json().unwrap();
        let ds2 = PreferenceDataset::from_json(&json).unwrap();
        assert_eq!(ds2.len(), 1);
        assert_eq!(ds2.pairs[0].prompt, "prompt");
    }
}
