//! Chunk importance scoring.
//!
//! Combines relevance, recency, and uniqueness signals into a single score
//! used to rank context chunks for inclusion in an LLM prompt.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// A scored context chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkScore {
    /// Index into the original chunk list.
    pub index: usize,
    /// The chunk text.
    pub text: String,
    /// Approximate token count for this chunk.
    pub tokens: usize,
    /// Relevance score (0.0-1.0) from retrieval / embedding similarity.
    pub relevance: f32,
    /// Recency score (0.0-1.0), newer = higher.
    pub recency: f32,
    /// Uniqueness score (0.0-1.0), less duplicate = higher.
    pub uniqueness: f32,
    /// Weighted combination of all signals.
    pub combined: f32,
}

/// Weights for the three scoring signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    pub relevance: f32,
    pub recency: f32,
    pub uniqueness: f32,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            relevance: 0.6,
            recency: 0.2,
            uniqueness: 0.2,
        }
    }
}

/// Score a list of chunks by importance.
///
/// Each input chunk is a `(text, retrieval_score)` pair.
/// A token-counting function is passed in so scoring is tokenizer-agnostic.
pub fn score_chunks(
    chunks: &[(String, f32)],
    weights: &ScoringWeights,
    count_tokens: &dyn Fn(&str) -> usize,
) -> Vec<ChunkScore> {
    if chunks.is_empty() {
        return Vec::new();
    }

    let n = chunks.len();

    // Pre-compute word sets for uniqueness
    let word_sets: Vec<HashSet<&str>> = chunks
        .iter()
        .map(|(text, _)| text.split_whitespace().collect())
        .collect();

    let mut scores = Vec::with_capacity(n);

    for (i, (text, retrieval_score)) in chunks.iter().enumerate() {
        let relevance = retrieval_score.clamp(0.0, 1.0);

        // Recency: later position = more recent = higher score
        let recency = if n == 1 {
            1.0
        } else {
            i as f32 / (n - 1) as f32
        };

        // Uniqueness: average (1 - jaccard) with all other chunks
        let uniqueness = if n == 1 {
            1.0
        } else {
            let total_distance: f32 = (0..n)
                .filter(|&j| j != i)
                .map(|j| 1.0 - jaccard_similarity(&word_sets[i], &word_sets[j]))
                .sum();
            total_distance / (n - 1) as f32
        };

        let combined = weights.relevance * relevance
            + weights.recency * recency
            + weights.uniqueness * uniqueness;

        let tokens = count_tokens(text);

        scores.push(ChunkScore {
            index: i,
            text: text.clone(),
            tokens,
            relevance,
            recency,
            uniqueness,
            combined,
        });
    }

    scores
}

/// Jaccard similarity between two word sets.
fn jaccard_similarity(a: &HashSet<&str>, b: &HashSet<&str>) -> f32 {
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f32 / union as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_counter(text: &str) -> usize {
        text.split_whitespace().count()
    }

    #[test]
    fn test_empty_chunks() {
        let scores = score_chunks(&[], &ScoringWeights::default(), &simple_counter);
        assert!(scores.is_empty());
    }

    #[test]
    fn test_single_chunk() {
        let chunks = vec![("hello world".into(), 0.9)];
        let scores = score_chunks(&chunks, &ScoringWeights::default(), &simple_counter);
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].relevance, 0.9);
        assert_eq!(scores[0].recency, 1.0);
        assert_eq!(scores[0].uniqueness, 1.0);
    }

    #[test]
    fn test_relevance_ordering() {
        let chunks = vec![
            ("low relevance chunk".into(), 0.1),
            ("high relevance chunk".into(), 0.9),
        ];
        let weights = ScoringWeights {
            relevance: 1.0,
            recency: 0.0,
            uniqueness: 0.0,
        };
        let scores = score_chunks(&chunks, &weights, &simple_counter);
        assert!(scores[1].combined > scores[0].combined);
    }

    #[test]
    fn test_recency_ordering() {
        let chunks = vec![("old chunk".into(), 0.5), ("new chunk".into(), 0.5)];
        let weights = ScoringWeights {
            relevance: 0.0,
            recency: 1.0,
            uniqueness: 0.0,
        };
        let scores = score_chunks(&chunks, &weights, &simple_counter);
        assert!(scores[1].combined > scores[0].combined);
    }

    #[test]
    fn test_uniqueness_scoring() {
        let chunks = vec![
            ("the cat sat on the mat".into(), 0.5),
            ("the cat sat on the mat".into(), 0.5), // duplicate
            ("a completely different sentence here".into(), 0.5),
        ];
        let weights = ScoringWeights {
            relevance: 0.0,
            recency: 0.0,
            uniqueness: 1.0,
        };
        let scores = score_chunks(&chunks, &weights, &simple_counter);
        // The unique chunk should score higher than the duplicates
        assert!(scores[2].uniqueness > scores[0].uniqueness);
    }

    #[test]
    fn test_token_counting() {
        let chunks = vec![("one two three four five".into(), 0.5)];
        let scores = score_chunks(&chunks, &ScoringWeights::default(), &simple_counter);
        assert_eq!(scores[0].tokens, 5);
    }

    #[test]
    fn test_clamped_relevance() {
        let chunks = vec![("text".into(), 1.5)];
        let scores = score_chunks(&chunks, &ScoringWeights::default(), &simple_counter);
        assert_eq!(scores[0].relevance, 1.0);
    }
}
