// ── N05: Fusion Engine ──────────────────────────────────────────────
use crate::modality::MultimodalEmbedding;

/// Errors from fusion operations.
#[derive(Debug, Clone, PartialEq)]
pub enum FusionError {
    EmptyInput,
    DimensionMismatch { expected: usize, got: usize },
    WeightCountMismatch { embeddings: usize, weights: usize },
}

impl std::fmt::Display for FusionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FusionError::EmptyInput => write!(f, "empty input"),
            FusionError::DimensionMismatch { expected, got } => {
                write!(f, "dimension mismatch: expected {expected}, got {got}")
            }
            FusionError::WeightCountMismatch { embeddings, weights } => {
                write!(f, "weight count mismatch: {embeddings} embeddings vs {weights} weights")
            }
        }
    }
}

impl std::error::Error for FusionError {}

/// Weighted average of embedding vectors.
/// All embeddings must have the same dimensionality.
pub fn weighted_average(
    embeddings: &[MultimodalEmbedding],
    weights: &[f32],
) -> Result<Vec<f32>, FusionError> {
    if embeddings.is_empty() {
        return Err(FusionError::EmptyInput);
    }
    if embeddings.len() != weights.len() {
        return Err(FusionError::WeightCountMismatch {
            embeddings: embeddings.len(),
            weights: weights.len(),
        });
    }

    let dim = embeddings[0].dim();
    for emb in &embeddings[1..] {
        if emb.dim() != dim {
            return Err(FusionError::DimensionMismatch {
                expected: dim,
                got: emb.dim(),
            });
        }
    }

    let weight_sum: f32 = weights.iter().sum();
    let norm = if weight_sum.abs() < f32::EPSILON {
        1.0
    } else {
        weight_sum
    };

    let mut result = vec![0.0f32; dim];
    for (emb, &w) in embeddings.iter().zip(weights.iter()) {
        for (i, &v) in emb.embedding.iter().enumerate() {
            result[i] += v * w;
        }
    }
    for v in &mut result {
        *v /= norm;
    }

    Ok(result)
}

/// Concatenate embedding vectors from different modalities.
pub fn concatenate(embeddings: &[MultimodalEmbedding]) -> Result<Vec<f32>, FusionError> {
    if embeddings.is_empty() {
        return Err(FusionError::EmptyInput);
    }
    let total_dim: usize = embeddings.iter().map(|e| e.dim()).sum();
    let mut result = Vec::with_capacity(total_dim);
    for emb in embeddings {
        result.extend_from_slice(&emb.embedding);
    }
    Ok(result)
}

/// Fusion engine that holds configuration and provides fusion methods.
pub struct FusionEngine {
    pub default_weights: Vec<f32>,
}

impl FusionEngine {
    pub fn new() -> Self {
        Self {
            default_weights: Vec::new(),
        }
    }

    pub fn with_default_weights(mut self, weights: Vec<f32>) -> Self {
        self.default_weights = weights;
        self
    }

    /// Fuse using weighted average with default weights.
    pub fn fuse_weighted(&self, embeddings: &[MultimodalEmbedding]) -> Result<Vec<f32>, FusionError> {
        let weights = if self.default_weights.len() == embeddings.len() {
            &self.default_weights
        } else {
            // Equal weights fallback
            return weighted_average(
                embeddings,
                &vec![1.0; embeddings.len()],
            );
        };
        weighted_average(embeddings, weights)
    }

    /// Fuse by concatenation.
    pub fn fuse_concat(&self, embeddings: &[MultimodalEmbedding]) -> Result<Vec<f32>, FusionError> {
        concatenate(embeddings)
    }
}

impl Default for FusionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modality::Modality;

    fn make_emb(modality: Modality, values: Vec<f32>) -> MultimodalEmbedding {
        MultimodalEmbedding::new(modality, values)
    }

    #[test]
    fn weighted_average_basic() {
        let embs = vec![
            make_emb(Modality::Text, vec![1.0, 0.0]),
            make_emb(Modality::Image, vec![0.0, 1.0]),
        ];
        let result = weighted_average(&embs, &[0.5, 0.5]).unwrap();
        assert_eq!(result.len(), 2);
        assert!((result[0] - 0.5).abs() < 1e-5);
        assert!((result[1] - 0.5).abs() < 1e-5);
    }

    #[test]
    fn weighted_average_unequal_weights() {
        let embs = vec![
            make_emb(Modality::Text, vec![2.0, 0.0]),
            make_emb(Modality::Image, vec![0.0, 4.0]),
        ];
        let result = weighted_average(&embs, &[0.75, 0.25]).unwrap();
        // (2.0*0.75 + 0.0*0.25) / 1.0 = 1.5
        assert!((result[0] - 1.5).abs() < 1e-5);
        // (0.0*0.75 + 4.0*0.25) / 1.0 = 1.0
        assert!((result[1] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn weighted_average_dimension_mismatch() {
        let embs = vec![
            make_emb(Modality::Text, vec![1.0, 2.0]),
            make_emb(Modality::Image, vec![1.0, 2.0, 3.0]),
        ];
        let err = weighted_average(&embs, &[0.5, 0.5]).unwrap_err();
        assert_eq!(err, FusionError::DimensionMismatch { expected: 2, got: 3 });
    }

    #[test]
    fn concatenation() {
        let embs = vec![
            make_emb(Modality::Text, vec![1.0, 2.0]),
            make_emb(Modality::Audio, vec![3.0, 4.0, 5.0]),
        ];
        let result = concatenate(&embs).unwrap();
        assert_eq!(result, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn empty_input_error() {
        assert_eq!(
            weighted_average(&[], &[]).unwrap_err(),
            FusionError::EmptyInput,
        );
        assert_eq!(
            concatenate(&[]).unwrap_err(),
            FusionError::EmptyInput,
        );
    }
}
