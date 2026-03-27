use async_trait::async_trait;
use std::fmt;
use tokio::sync::RwLock;

use crate::chunk::{Chunk, EmbeddedChunk};

/// Result of a vector similarity search.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub chunk: Chunk,
    pub score: f32,
}

/// Errors from vector store operations.
#[derive(Debug)]
pub enum StoreError {
    Internal(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Internal(msg) => write!(f, "store error: {}", msg),
        }
    }
}

impl std::error::Error for StoreError {}

/// Trait for vector storage backends.
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Upsert embedded chunks into the store.
    async fn upsert(&self, chunks: &[EmbeddedChunk]) -> Result<(), StoreError>;

    /// Search for the top-k most similar chunks to the query embedding.
    async fn search(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<SearchResult>, StoreError>;

    /// Delete all chunks belonging to a document. Returns count deleted.
    async fn delete_by_doc(&self, doc_id: &str) -> Result<usize, StoreError>;

    /// Return the total number of stored chunks.
    async fn count(&self) -> Result<usize, StoreError>;
}

// ---------------------------------------------------------------------------
// InMemoryStore — brute-force cosine similarity
// ---------------------------------------------------------------------------

/// In-memory vector store using brute-force cosine similarity.
/// Suitable for development and testing; production should use Qdrant or similar.
pub struct InMemoryStore {
    chunks: RwLock<Vec<EmbeddedChunk>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            chunks: RwLock::new(Vec::new()),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut mag_a = 0.0f32;
    let mut mag_b = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        mag_a += a[i] * a[i];
        mag_b += b[i] * b[i];
    }
    let denom = mag_a.sqrt() * mag_b.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        (dot / denom).clamp(0.0, 1.0)
    }
}

#[async_trait]
impl VectorStore for InMemoryStore {
    async fn upsert(&self, new_chunks: &[EmbeddedChunk]) -> Result<(), StoreError> {
        let mut store = self.chunks.write().await;
        for nc in new_chunks {
            // Replace existing chunk with same ID, or append
            if let Some(pos) = store.iter().position(|c| c.chunk.id == nc.chunk.id) {
                store[pos] = nc.clone();
            } else {
                store.push(nc.clone());
            }
        }
        Ok(())
    }

    async fn search(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<SearchResult>, StoreError> {
        let store = self.chunks.read().await;
        let mut scored: Vec<(f32, &EmbeddedChunk)> = store
            .iter()
            .map(|ec| (cosine_similarity(query_embedding, &ec.embedding), ec))
            .collect();

        // Sort descending by score
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored
            .into_iter()
            .take(top_k)
            .map(|(score, ec)| SearchResult {
                chunk: ec.chunk.clone(),
                score,
            })
            .collect())
    }

    async fn delete_by_doc(&self, doc_id: &str) -> Result<usize, StoreError> {
        let mut store = self.chunks.write().await;
        let before = store.len();
        store.retain(|ec| ec.chunk.doc_id != doc_id);
        Ok(before - store.len())
    }

    async fn count(&self) -> Result<usize, StoreError> {
        Ok(self.chunks.read().await.len())
    }
}

// ==========================================================================
// Tests
// ==========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::Chunk;

    fn make_embedded(id: &str, doc_id: &str, embedding: Vec<f32>) -> EmbeddedChunk {
        EmbeddedChunk {
            chunk: Chunk {
                id: id.to_string(),
                doc_id: doc_id.to_string(),
                content: format!("content of {}", id),
                index: 0,
                metadata: serde_json::json!({}),
            },
            embedding,
        }
    }

    #[tokio::test]
    async fn upsert_and_count() {
        let store = InMemoryStore::new();
        assert_eq!(store.count().await.unwrap(), 0);

        let chunks = vec![
            make_embedded("c1", "doc1", vec![1.0, 0.0, 0.0]),
            make_embedded("c2", "doc1", vec![0.0, 1.0, 0.0]),
        ];
        store.upsert(&chunks).await.unwrap();
        assert_eq!(store.count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn upsert_replaces_existing() {
        let store = InMemoryStore::new();
        let chunks = vec![make_embedded("c1", "doc1", vec![1.0, 0.0])];
        store.upsert(&chunks).await.unwrap();

        // Upsert same ID with different embedding
        let chunks2 = vec![make_embedded("c1", "doc1", vec![0.0, 1.0])];
        store.upsert(&chunks2).await.unwrap();
        assert_eq!(store.count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn search_cosine_similarity() {
        let store = InMemoryStore::new();
        let chunks = vec![
            make_embedded("c1", "doc1", vec![1.0, 0.0, 0.0]),
            make_embedded("c2", "doc1", vec![0.0, 1.0, 0.0]),
            make_embedded("c3", "doc1", vec![0.7, 0.7, 0.0]),
        ];
        store.upsert(&chunks).await.unwrap();

        // Query close to c1
        let results = store.search(&[1.0, 0.0, 0.0], 2).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].chunk.id, "c1");
        assert!((results[0].score - 1.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn delete_by_doc() {
        let store = InMemoryStore::new();
        let chunks = vec![
            make_embedded("c1", "doc1", vec![1.0]),
            make_embedded("c2", "doc2", vec![0.5]),
            make_embedded("c3", "doc1", vec![0.3]),
        ];
        store.upsert(&chunks).await.unwrap();
        assert_eq!(store.count().await.unwrap(), 3);

        let deleted = store.delete_by_doc("doc1").await.unwrap();
        assert_eq!(deleted, 2);
        assert_eq!(store.count().await.unwrap(), 1);
    }

    #[test]
    fn cosine_similarity_identical() {
        let score = cosine_similarity(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]);
        assert!((score - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_similarity_orthogonal() {
        let score = cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]);
        assert!(score.abs() < 1e-5);
    }
}
