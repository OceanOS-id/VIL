use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// A single indexed chunk with its embedding vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedChunk {
    pub text: String,
    pub embedding: Vec<f32>,
    pub index: usize,
}

/// Result returned from a search query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResult {
    pub text: String,
    pub index: usize,
    pub score: f32,
}

/// Append-only index that stores chunks with embeddings, protected by RwLock.
pub struct IndexWriter {
    chunks: RwLock<Vec<IndexedChunk>>,
}

impl IndexWriter {
    pub fn new() -> Self {
        Self {
            chunks: RwLock::new(Vec::new()),
        }
    }

    /// Append a chunk with its embedding to the index.
    pub fn append(&self, text: String, embedding: Vec<f32>) -> usize {
        let mut chunks = self.chunks.write();
        let index = chunks.len();
        chunks.push(IndexedChunk { text, embedding, index });
        index
    }

    /// Total number of indexed chunks.
    pub fn len(&self) -> usize {
        self.chunks.read().len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.chunks.read().is_empty()
    }

    /// Brute-force cosine similarity search. Returns top_k results sorted by
    /// descending score.
    pub fn search(&self, query_embedding: &[f32], top_k: usize) -> Vec<StreamResult> {
        let chunks = self.chunks.read();
        if chunks.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<StreamResult> = chunks
            .iter()
            .map(|chunk| {
                let score = cosine_similarity(query_embedding, &chunk.embedding);
                StreamResult {
                    text: chunk.text.clone(),
                    index: chunk.index,
                    score,
                }
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }

    /// Return all indexed chunks (cloned).
    pub fn all_chunks(&self) -> Vec<IndexedChunk> {
        self.chunks.read().clone()
    }
}

impl Default for IndexWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    dot / (mag_a * mag_b)
}
