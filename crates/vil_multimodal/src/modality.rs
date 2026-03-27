// ── N05: Modality Types ─────────────────────────────────────────────
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported modality types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Modality {
    Text,
    Image,
    Audio,
    Video,
}

impl std::fmt::Display for Modality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Modality::Text => write!(f, "text"),
            Modality::Image => write!(f, "image"),
            Modality::Audio => write!(f, "audio"),
            Modality::Video => write!(f, "video"),
        }
    }
}

/// An embedding tagged with its source modality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultimodalEmbedding {
    pub modality: Modality,
    pub embedding: Vec<f32>,
    pub metadata: HashMap<String, String>,
}

impl MultimodalEmbedding {
    pub fn new(modality: Modality, embedding: Vec<f32>) -> Self {
        Self {
            modality,
            embedding,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Dimensionality of the embedding vector.
    pub fn dim(&self) -> usize {
        self.embedding.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_embedding() {
        let emb = MultimodalEmbedding::new(Modality::Text, vec![1.0, 2.0, 3.0]);
        assert_eq!(emb.modality, Modality::Text);
        assert_eq!(emb.dim(), 3);
    }

    #[test]
    fn embedding_with_metadata() {
        let emb = MultimodalEmbedding::new(Modality::Image, vec![0.5; 4])
            .with_metadata("source", "camera_01");
        assert_eq!(emb.metadata["source"], "camera_01");
    }
}
