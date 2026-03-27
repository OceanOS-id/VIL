/// Distance metric used for vector similarity comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DistanceMetric {
    /// Cosine distance: 0 = identical, 2 = opposite.
    Cosine,
    /// Negative dot product: more negative = more similar.
    DotProduct,
    /// Euclidean (L2) distance: 0 = identical.
    Euclidean,
}

/// Compute distance between two vectors using the given metric.
///
/// # Panics
/// Panics if `a` and `b` have different lengths.
pub fn distance(a: &[f32], b: &[f32], metric: DistanceMetric) -> f32 {
    assert_eq!(a.len(), b.len(), "vector dimensions must match");
    match metric {
        DistanceMetric::Cosine => 1.0 - cosine_similarity(a, b),
        DistanceMetric::DotProduct => -dot_product(a, b),
        DistanceMetric::Euclidean => euclidean_distance(a, b),
    }
}

/// Cosine similarity in [-1, 1]. Returns 0.0 if either vector has zero magnitude.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

/// Dot product of two vectors.
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Euclidean (L2) distance between two vectors.
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f32>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];
        let d = distance(&v, &v, DistanceMetric::Cosine);
        assert!(d.abs() < 1e-6, "identical vectors should have cosine distance ~0, got {d}");
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let d = distance(&a, &b, DistanceMetric::Cosine);
        assert!((d - 1.0).abs() < 1e-6, "orthogonal vectors should have cosine distance ~1, got {d}");
    }

    #[test]
    fn cosine_opposite_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let d = distance(&a, &b, DistanceMetric::Cosine);
        assert!((d - 2.0).abs() < 1e-6, "opposite vectors should have cosine distance ~2, got {d}");
    }

    #[test]
    fn dot_product_basic() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let dp = dot_product(&a, &b);
        assert!((dp - 32.0).abs() < 1e-6);
        // Distance metric returns negative dot product
        let d = distance(&a, &b, DistanceMetric::DotProduct);
        assert!((d - (-32.0)).abs() < 1e-6);
    }

    #[test]
    fn euclidean_basic() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        let d = distance(&a, &b, DistanceMetric::Euclidean);
        assert!((d - 5.0).abs() < 1e-6);
    }

    #[test]
    fn euclidean_identical() {
        let v = vec![1.0, 2.0, 3.0];
        let d = distance(&v, &v, DistanceMetric::Euclidean);
        assert!(d.abs() < 1e-6);
    }

    #[test]
    fn cosine_zero_vector() {
        let a = vec![0.0, 0.0];
        let b = vec![1.0, 2.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6, "zero vector cosine similarity should be 0");
    }
}
