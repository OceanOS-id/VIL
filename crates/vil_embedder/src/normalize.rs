/// L2-normalize a vector in place so its magnitude becomes 1.0.
///
/// If the vector has zero magnitude, it is left unchanged.
pub fn l2_normalize(v: &mut [f32]) {
    let mag: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag > 0.0 {
        v.iter_mut().for_each(|x| *x /= mag);
    }
}

/// L2-normalize every vector in the batch.
pub fn l2_normalize_batch(vectors: &mut [Vec<f32>]) {
    for v in vectors.iter_mut() {
        l2_normalize(v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_to_unit_length() {
        let mut v = vec![3.0, 4.0];
        l2_normalize(&mut v);
        let mag: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (mag - 1.0).abs() < 1e-6,
            "magnitude after normalize should be 1.0, got {mag}"
        );
        assert!((v[0] - 0.6).abs() < 1e-6);
        assert!((v[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn normalize_already_unit() {
        let mut v = vec![1.0, 0.0, 0.0];
        l2_normalize(&mut v);
        assert!((v[0] - 1.0).abs() < 1e-6);
        assert!(v[1].abs() < 1e-6);
        assert!(v[2].abs() < 1e-6);
    }

    #[test]
    fn normalize_zero_vector_unchanged() {
        let mut v = vec![0.0, 0.0, 0.0];
        l2_normalize(&mut v);
        assert_eq!(v, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn normalize_single_element() {
        let mut v = vec![5.0];
        l2_normalize(&mut v);
        assert!((v[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn normalize_negative_values() {
        let mut v = vec![-3.0, -4.0];
        l2_normalize(&mut v);
        let mag: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((mag - 1.0).abs() < 1e-6);
        assert!((v[0] - (-0.6)).abs() < 1e-6);
        assert!((v[1] - (-0.8)).abs() < 1e-6);
    }

    #[test]
    fn batch_normalize() {
        let mut vectors = vec![vec![3.0, 4.0], vec![0.0, 0.0], vec![1.0, 0.0]];
        l2_normalize_batch(&mut vectors);

        // First vector: [0.6, 0.8]
        let mag0: f32 = vectors[0].iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((mag0 - 1.0).abs() < 1e-6);

        // Second vector: still zero
        assert_eq!(vectors[1], vec![0.0, 0.0]);

        // Third vector: already unit
        assert!((vectors[2][0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn batch_normalize_empty() {
        let mut vectors: Vec<Vec<f32>> = vec![];
        l2_normalize_batch(&mut vectors);
        assert!(vectors.is_empty());
    }
}
