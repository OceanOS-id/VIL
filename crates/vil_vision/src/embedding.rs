use async_trait::async_trait;

use crate::analyzer::VisionError;

/// Trait for generating vector embeddings from images.
#[async_trait]
pub trait ImageEmbedder: Send + Sync {
    /// Generate a vector embedding for the given image bytes.
    async fn embed_image(&self, image: &[u8]) -> Result<Vec<f32>, VisionError>;

    /// The dimensionality of the output embedding vector.
    fn dimension(&self) -> usize;

    /// Name of this embedder backend.
    fn name(&self) -> &str;
}

/// A no-op embedder that returns an error — extend with real backend (CLIP, SentenceTransformers, etc.).
pub struct NoopEmbedder {
    dim: usize,
}

impl NoopEmbedder {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }
}

impl Default for NoopEmbedder {
    fn default() -> Self {
        Self::new(512)
    }
}

#[async_trait]
impl ImageEmbedder for NoopEmbedder {
    async fn embed_image(&self, image: &[u8]) -> Result<Vec<f32>, VisionError> {
        if image.is_empty() {
            return Err(VisionError::EmptyImage);
        }
        Err(VisionError::AnalysisFailed(
            "no embedding backend configured".into(),
        ))
    }

    fn dimension(&self) -> usize {
        self.dim
    }

    fn name(&self) -> &str {
        "noop"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_embedder_empty() {
        let e = NoopEmbedder::default();
        let result = e.embed_image(b"").await;
        assert!(matches!(result, Err(VisionError::EmptyImage)));
    }

    #[tokio::test]
    async fn test_noop_embedder_dimension() {
        let e = NoopEmbedder::new(768);
        assert_eq!(e.dimension(), 768);
    }
}
