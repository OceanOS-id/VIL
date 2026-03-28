//! Result merger — combine and deduplicate results from multiple sources.

use crate::source::SourceResult;

/// Merged federated result.
#[derive(Debug, Clone)]
pub struct FederatedResult {
    pub results: Vec<SourceResult>,
    pub sources_queried: usize,
    pub total_ms: u64,
}

/// Merges results from multiple sources.
pub struct ResultMerger {
    /// Similarity threshold (0.0–1.0) for deduplication.
    pub dedup_threshold: f32,
}

impl Default for ResultMerger {
    fn default() -> Self {
        Self {
            dedup_threshold: 0.85,
        }
    }
}

impl ResultMerger {
    pub fn new(dedup_threshold: f32) -> Self {
        Self { dedup_threshold }
    }

    /// Merge multiple result sets: interleave by score, deduplicate similar.
    pub fn merge(&self, result_sets: Vec<Vec<SourceResult>>) -> Vec<SourceResult> {
        // Flatten and sort by score descending.
        let mut all: Vec<SourceResult> = result_sets.into_iter().flatten().collect();
        all.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Deduplicate similar results.
        let mut deduped: Vec<SourceResult> = Vec::new();
        for item in all {
            let dominated = deduped.iter().any(|existing| {
                self.text_similarity(&existing.text, &item.text) >= self.dedup_threshold
            });
            if !dominated {
                deduped.push(item);
            }
        }

        deduped
    }

    /// Simple Jaccard-like word-overlap similarity.
    fn text_similarity(&self, a: &str, b: &str) -> f32 {
        let words_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
        let words_b: std::collections::HashSet<&str> = b.split_whitespace().collect();

        if words_a.is_empty() && words_b.is_empty() {
            return 1.0;
        }

        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();

        if union == 0 {
            return 0.0;
        }

        intersection as f32 / union as f32
    }
}
