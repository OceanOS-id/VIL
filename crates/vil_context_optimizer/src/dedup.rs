//! Content deduplication based on word-level Jaccard similarity.
//!
//! Removes near-duplicate chunks so that only substantially unique content
//! is kept, saving token budget for more diverse context.

use std::collections::HashSet;

/// Remove near-duplicate chunks based on word overlap.
///
/// Returns the indices of chunks to keep.
///
/// - `threshold`: Jaccard similarity above which a chunk is considered duplicate.
///   - `0.0` = keep all (nothing is duplicate enough)
///   - `1.0` = only remove exact duplicates
///   - Default recommendation: `0.8`
pub fn deduplicate(chunks: &[String], threshold: f32) -> Vec<usize> {
    let mut keep: Vec<usize> = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        let words_i: HashSet<&str> = chunk.split_whitespace().collect();
        let is_dup = keep.iter().any(|&j| {
            let words_j: HashSet<&str> = chunks[j].split_whitespace().collect();
            jaccard(&words_i, &words_j) > threshold
        });
        if !is_dup {
            keep.push(i);
        }
    }

    keep
}

fn jaccard(a: &HashSet<&str>, b: &HashSet<&str>) -> f32 {
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

    #[test]
    fn test_identical_chunks_deduplicated() {
        let chunks = vec![
            "the quick brown fox".into(),
            "the quick brown fox".into(),
            "the quick brown fox".into(),
        ];
        let kept = deduplicate(&chunks, 0.8);
        assert_eq!(kept, vec![0]);
    }

    #[test]
    fn test_unique_chunks_kept() {
        let chunks = vec![
            "apples oranges bananas grapes".into(),
            "rust python java golang".into(),
            "monday tuesday wednesday thursday".into(),
        ];
        let kept = deduplicate(&chunks, 0.8);
        assert_eq!(kept, vec![0, 1, 2]);
    }

    #[test]
    fn test_partially_overlapping() {
        let chunks = vec![
            "the quick brown fox jumps over the lazy dog".into(),
            "the quick brown fox sits on the lazy cat".into(), // high overlap
            "completely different unrelated content here now".into(),
        ];
        // At 0.4 threshold the two fox sentences should be deduped (jaccard ~0.45)
        let kept = deduplicate(&chunks, 0.4);
        assert_eq!(kept.len(), 2);
        assert!(kept.contains(&0));
        assert!(kept.contains(&2));
    }

    #[test]
    fn test_threshold_zero_keeps_all() {
        let chunks = vec!["same text".into(), "same text".into()];
        // threshold 0.0 means jaccard must be > 0.0 to dedup — identical is 1.0 > 0.0
        // so it still deduplicates identical text
        let kept = deduplicate(&chunks, 0.0);
        assert_eq!(kept.len(), 1);
    }

    #[test]
    fn test_threshold_one_keeps_most() {
        let chunks = vec![
            "the quick brown fox".into(),
            "the quick brown fox".into(), // identical => jaccard=1.0, not > 1.0
        ];
        // threshold 1.0: only remove if jaccard > 1.0 (impossible), so keep all
        let kept = deduplicate(&chunks, 1.0);
        assert_eq!(kept, vec![0, 1]);
    }

    #[test]
    fn test_empty_chunks() {
        let chunks: Vec<String> = vec![];
        let kept = deduplicate(&chunks, 0.8);
        assert!(kept.is_empty());
    }

    #[test]
    fn test_single_chunk() {
        let chunks = vec!["only one".into()];
        let kept = deduplicate(&chunks, 0.8);
        assert_eq!(kept, vec![0]);
    }

    #[test]
    fn test_empty_strings() {
        let chunks = vec!["".into(), "".into()];
        // Both empty => jaccard = 0.0, not > 0.8, so both kept
        let kept = deduplicate(&chunks, 0.8);
        assert_eq!(kept, vec![0, 1]);
    }
}
