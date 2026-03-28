// ── N02: Quality Checker ────────────────────────────────────────────
use std::collections::HashSet;

/// Heuristic quality checker for synthetic examples.
#[derive(Debug, Clone)]
pub struct QualityChecker {
    pub min_length: usize,
    pub max_similarity_to_seed: f64,
}

impl Default for QualityChecker {
    fn default() -> Self {
        Self {
            min_length: 10,
            max_similarity_to_seed: 0.95,
        }
    }
}

impl QualityChecker {
    pub fn new(min_length: usize, max_similarity_to_seed: f64) -> Self {
        Self {
            min_length,
            max_similarity_to_seed,
        }
    }

    /// Compute a quality score (0.0–1.0) for a generated example.
    /// Factors: length adequacy, diversity from seeds, coherence heuristic.
    pub fn score(&self, text: &str, seed_texts: &[String]) -> f64 {
        if text.is_empty() {
            return 0.0;
        }

        // Length score
        let length_score = if text.len() >= self.min_length {
            1.0
        } else {
            text.len() as f64 / self.min_length as f64
        };

        // Diversity score — not too similar to any seed
        let diversity_score = if seed_texts.is_empty() {
            1.0
        } else {
            let max_sim = seed_texts
                .iter()
                .map(|s| jaccard_similarity(text, s))
                .fold(0.0f64, f64::max);
            if max_sim >= self.max_similarity_to_seed {
                0.1 // penalise near-copies
            } else {
                1.0 - (max_sim * 0.5)
            }
        };

        // Coherence heuristic — has words, not just symbols
        let alnum_ratio =
            text.chars().filter(|c| c.is_alphanumeric()).count() as f64 / text.len() as f64;
        let coherence_score = alnum_ratio.min(1.0);

        (length_score * 0.3 + diversity_score * 0.4 + coherence_score * 0.3).min(1.0)
    }

    /// Returns true if the text passes minimum quality.
    pub fn passes(&self, text: &str, seed_texts: &[String], threshold: f64) -> bool {
        self.score(text, seed_texts) >= threshold
    }
}

fn jaccard_similarity(a: &str, b: &str) -> f64 {
    let set_a: HashSet<&str> = a.split_whitespace().collect();
    let set_b: HashSet<&str> = b.split_whitespace().collect();
    if set_a.is_empty() && set_b.is_empty() {
        return 1.0;
    }
    let intersection = set_a.intersection(&set_b).count() as f64;
    let union = set_a.union(&set_b).count() as f64;
    if union == 0.0 {
        1.0
    } else {
        intersection / union
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_score_good_text() {
        let qc = QualityChecker::default();
        let score = qc.score(
            "This is a well-formed synthetic example with good content.",
            &[],
        );
        assert!(score > 0.5);
    }

    #[test]
    fn quality_score_empty() {
        let qc = QualityChecker::default();
        assert_eq!(qc.score("", &[]), 0.0);
    }

    #[test]
    fn quality_penalises_copy() {
        let qc = QualityChecker::new(5, 0.8);
        let seed = vec!["the quick brown fox jumps over the lazy dog".into()];
        let copy_score = qc.score("the quick brown fox jumps over the lazy dog", &seed);
        let diff_score = qc.score("a completely different sentence about programming", &seed);
        assert!(diff_score > copy_score);
    }
}
