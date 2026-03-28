/// Cosine similarity between two vectors.
///
/// Returns a value in `[-1.0, 1.0]`. If either vector has zero magnitude,
/// returns `0.0`.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    dot / (mag_a * mag_b)
}

/// Dot product of two vectors.
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Euclidean distance between two vectors.
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

/// Find the top-K most similar vectors to `query` by cosine similarity.
///
/// Returns a `Vec` of `(index, similarity_score)` pairs sorted descending
/// by similarity. The result is truncated to at most `k` entries.
pub fn top_k_similar(query: &[f32], candidates: &[Vec<f32>], k: usize) -> Vec<(usize, f32)> {
    let mut scored: Vec<(usize, f32)> = candidates
        .iter()
        .enumerate()
        .map(|(i, c)| (i, cosine_similarity(query, c)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(k);
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── cosine_similarity ────────────────────────────────────────────

    #[test]
    fn cosine_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!(
            (sim - 1.0).abs() < 1e-6,
            "identical vectors should be 1.0, got {sim}"
        );
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            sim.abs() < 1e-6,
            "orthogonal vectors should be 0.0, got {sim}"
        );
    }

    #[test]
    fn cosine_opposite_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            (sim + 1.0).abs() < 1e-6,
            "opposite vectors should be -1.0, got {sim}"
        );
    }

    #[test]
    fn cosine_zero_vector() {
        let a = vec![1.0, 2.0];
        let b = vec![0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
        assert_eq!(cosine_similarity(&b, &a), 0.0);
        assert_eq!(cosine_similarity(&b, &b), 0.0);
    }

    #[test]
    fn cosine_scaled_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![2.0, 4.0, 6.0]; // same direction, scaled
        let sim = cosine_similarity(&a, &b);
        assert!(
            (sim - 1.0).abs() < 1e-6,
            "parallel vectors should be 1.0, got {sim}"
        );
    }

    #[test]
    fn cosine_empty_vectors() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        // Both magnitudes are 0, so result is 0.
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    // ── dot_product ──────────────────────────────────────────────────

    #[test]
    fn dot_product_basic() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        // 1*4 + 2*5 + 3*6 = 32
        assert!((dot_product(&a, &b) - 32.0).abs() < 1e-6);
    }

    #[test]
    fn dot_product_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((dot_product(&a, &b)).abs() < 1e-6);
    }

    #[test]
    fn dot_product_zero_vector() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![0.0, 0.0, 0.0];
        assert_eq!(dot_product(&a, &b), 0.0);
    }

    #[test]
    fn dot_product_empty() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        assert_eq!(dot_product(&a, &b), 0.0);
    }

    // ── euclidean_distance ───────────────────────────────────────────

    #[test]
    fn euclidean_same_point() {
        let v = vec![1.0, 2.0, 3.0];
        assert!((euclidean_distance(&v, &v)).abs() < 1e-6);
    }

    #[test]
    fn euclidean_known_distance() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        let dist = euclidean_distance(&a, &b);
        assert!((dist - 5.0).abs() < 1e-6, "expected 5.0, got {dist}");
    }

    #[test]
    fn euclidean_unit_distance() {
        let a = vec![0.0];
        let b = vec![1.0];
        assert!((euclidean_distance(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn euclidean_empty() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        assert_eq!(euclidean_distance(&a, &b), 0.0);
    }

    // ── top_k_similar ────────────────────────────────────────────────

    #[test]
    fn top_k_returns_correct_ordering() {
        let query = vec![1.0, 0.0];
        let candidates = vec![
            vec![0.0, 1.0],  // orthogonal → 0.0
            vec![1.0, 0.0],  // identical → 1.0
            vec![1.0, 1.0],  // 45 degrees → ~0.707
            vec![-1.0, 0.0], // opposite → -1.0
        ];

        let top2 = top_k_similar(&query, &candidates, 2);
        assert_eq!(top2.len(), 2);
        assert_eq!(top2[0].0, 1); // index of identical vector
        assert_eq!(top2[1].0, 2); // index of 45-degree vector
        assert!((top2[0].1 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn top_k_larger_than_candidates() {
        let query = vec![1.0, 0.0];
        let candidates = vec![vec![1.0, 0.0], vec![0.0, 1.0]];

        let top5 = top_k_similar(&query, &candidates, 5);
        assert_eq!(top5.len(), 2); // only 2 candidates exist
    }

    #[test]
    fn top_k_empty_candidates() {
        let query = vec![1.0, 0.0];
        let candidates: Vec<Vec<f32>> = vec![];

        let top = top_k_similar(&query, &candidates, 3);
        assert!(top.is_empty());
    }

    #[test]
    fn top_k_k_zero() {
        let query = vec![1.0, 0.0];
        let candidates = vec![vec![1.0, 0.0]];
        let top = top_k_similar(&query, &candidates, 0);
        assert!(top.is_empty());
    }
}
