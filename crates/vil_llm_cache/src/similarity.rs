use vil_embedder::similarity::cosine_similarity;

/// Find the best matching embedding above a similarity threshold.
///
/// Returns the index and similarity score of the best match, or `None`
/// if no candidate exceeds the threshold.
pub fn find_similar(
    query: &[f32],
    candidates: &[(Vec<f32>, usize)], // (embedding, index_into_responses)
    threshold: f32,
) -> Option<(usize, f32)> {
    let mut best: Option<(usize, f32)> = None;

    for (embedding, idx) in candidates {
        let sim = cosine_similarity(query, embedding);
        if sim >= threshold {
            match best {
                Some((_, best_sim)) if sim > best_sim => {
                    best = Some((*idx, sim));
                }
                None => {
                    best = Some((*idx, sim));
                }
                _ => {}
            }
        }
    }

    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_similar_exact_match() {
        let query = vec![1.0, 0.0, 0.0];
        let candidates = vec![(vec![1.0, 0.0, 0.0], 0)];
        let result = find_similar(&query, &candidates, 0.9);
        assert!(result.is_some());
        let (idx, sim) = result.unwrap();
        assert_eq!(idx, 0);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn find_similar_below_threshold() {
        let query = vec![1.0, 0.0];
        let candidates = vec![
            (vec![0.0, 1.0], 0), // orthogonal = 0.0
        ];
        assert!(find_similar(&query, &candidates, 0.5).is_none());
    }

    #[test]
    fn find_similar_best_of_multiple() {
        let query = vec![1.0, 0.0];
        let candidates = vec![
            (vec![0.9, 0.1], 0),
            (vec![1.0, 0.0], 1), // exact match
            (vec![0.8, 0.2], 2),
        ];
        let (idx, _) = find_similar(&query, &candidates, 0.5).unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn find_similar_empty_candidates() {
        let query = vec![1.0, 0.0];
        assert!(find_similar(&query, &[], 0.5).is_none());
    }
}
