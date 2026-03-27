use parking_lot::RwLock;
use serde::Serialize;
use vil_embedder::similarity::cosine_similarity;

/// Pre-built vector index optimized for sub-millisecond search.
/// All vectors stored in contiguous memory for cache locality.
pub struct RealtimeIndex {
    /// Flat vector storage: [vec0_dim0, vec0_dim1, ..., vec1_dim0, ...]
    vectors: RwLock<Vec<f32>>,
    /// Document metadata per vector.
    documents: RwLock<Vec<DocEntry>>,
    /// Embedding dimension.
    dimension: usize,
    /// Number of vectors.
    count: RwLock<usize>,
}

/// A document entry stored alongside its embedding.
#[derive(Clone, Debug)]
pub struct DocEntry {
    pub id: String,
    pub text: String,
    pub metadata: serde_json::Value,
}

/// A single search result.
#[derive(Serialize)]
pub struct RealtimeResult {
    pub doc_id: String,
    pub text: String,
    pub score: f32,
    pub metadata: serde_json::Value,
}

impl RealtimeIndex {
    /// Create a new empty index for vectors of the given dimension.
    pub fn new(dimension: usize) -> Self {
        Self {
            vectors: RwLock::new(Vec::new()),
            documents: RwLock::new(Vec::new()),
            dimension,
            count: RwLock::new(0),
        }
    }

    /// Add a pre-computed embedding with its document.
    ///
    /// # Panics
    /// Panics if `embedding.len() != self.dimension`.
    pub fn add(&self, embedding: &[f32], doc: DocEntry) {
        assert_eq!(
            embedding.len(),
            self.dimension,
            "embedding dimension mismatch: expected {}, got {}",
            self.dimension,
            embedding.len()
        );
        let mut vecs = self.vectors.write();
        let mut docs = self.documents.write();
        let mut cnt = self.count.write();

        vecs.extend_from_slice(embedding);
        docs.push(doc);
        *cnt += 1;
    }

    /// Brute-force search optimised for cache locality.
    ///
    /// Iterates over contiguous flat vector storage, computes cosine similarity
    /// for each document, and returns the top-K results sorted descending by
    /// score.
    pub fn search(&self, query_embedding: &[f32], top_k: usize) -> Vec<RealtimeResult> {
        let vecs = self.vectors.read();
        let docs = self.documents.read();
        let n = *self.count.read();

        if n == 0 || top_k == 0 {
            return Vec::new();
        }

        // Compute similarity for every stored vector.
        let mut scored: Vec<(usize, f32)> = (0..n)
            .map(|i| {
                let start = i * self.dimension;
                let end = start + self.dimension;
                let candidate = &vecs[start..end];
                (i, cosine_similarity(query_embedding, candidate))
            })
            .collect();

        // Partial sort: put top_k highest scores at the front.
        let k = top_k.min(n);
        scored.select_nth_unstable_by(k.saturating_sub(1), |a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(k);
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored
            .into_iter()
            .map(|(i, score)| {
                let doc = &docs[i];
                RealtimeResult {
                    doc_id: doc.id.clone(),
                    text: doc.text.clone(),
                    score,
                    metadata: doc.metadata.clone(),
                }
            })
            .collect()
    }

    /// Number of documents in the index.
    pub fn count(&self) -> usize {
        *self.count.read()
    }

    /// Remove all documents and embeddings.
    pub fn clear(&self) {
        self.vectors.write().clear();
        self.documents.write().clear();
        *self.count.write() = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_index(dim: usize) -> RealtimeIndex {
        RealtimeIndex::new(dim)
    }

    fn random_vec(dim: usize, seed: u64) -> Vec<f32> {
        // Simple deterministic pseudo-random via LCG.
        let mut state = seed;
        (0..dim)
            .map(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                // Map to [-1, 1]
                ((state >> 33) as f32) / (u32::MAX as f32) * 2.0 - 1.0
            })
            .collect()
    }

    #[test]
    fn add_and_count() {
        let idx = make_index(4);
        assert_eq!(idx.count(), 0);

        idx.add(
            &[1.0, 0.0, 0.0, 0.0],
            DocEntry {
                id: "d1".into(),
                text: "doc one".into(),
                metadata: serde_json::json!({}),
            },
        );
        assert_eq!(idx.count(), 1);
    }

    #[test]
    fn search_returns_sorted() {
        let idx = make_index(2);
        idx.add(
            &[0.0, 1.0],
            DocEntry {
                id: "orthogonal".into(),
                text: "orth".into(),
                metadata: serde_json::json!({}),
            },
        );
        idx.add(
            &[1.0, 0.0],
            DocEntry {
                id: "identical".into(),
                text: "same".into(),
                metadata: serde_json::json!({}),
            },
        );
        idx.add(
            &[0.7071, 0.7071],
            DocEntry {
                id: "mid".into(),
                text: "mid".into(),
                metadata: serde_json::json!({}),
            },
        );

        let results = idx.search(&[1.0, 0.0], 3);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].doc_id, "identical");
        assert_eq!(results[1].doc_id, "mid");
        assert_eq!(results[2].doc_id, "orthogonal");
    }

    #[test]
    fn empty_search() {
        let idx = make_index(4);
        let results = idx.search(&[1.0, 0.0, 0.0, 0.0], 5);
        assert!(results.is_empty());
    }

    #[test]
    fn clear_index() {
        let idx = make_index(2);
        idx.add(
            &[1.0, 0.0],
            DocEntry {
                id: "a".into(),
                text: "a".into(),
                metadata: serde_json::json!({}),
            },
        );
        assert_eq!(idx.count(), 1);
        idx.clear();
        assert_eq!(idx.count(), 0);
        assert!(idx.search(&[1.0, 0.0], 5).is_empty());
    }

    #[test]
    fn identical_vectors_score_highest() {
        let idx = make_index(384);
        let target = random_vec(384, 42);
        idx.add(
            &target,
            DocEntry {
                id: "target".into(),
                text: "target".into(),
                metadata: serde_json::json!({}),
            },
        );
        // Add 99 other random docs.
        for i in 1..100u64 {
            let v = random_vec(384, 100 + i);
            idx.add(
                &v,
                DocEntry {
                    id: format!("other_{i}"),
                    text: format!("other {i}"),
                    metadata: serde_json::json!({}),
                },
            );
        }
        let results = idx.search(&target, 1);
        assert_eq!(results[0].doc_id, "target");
        assert!((results[0].score - 1.0).abs() < 1e-5);
    }

    #[test]
    fn search_100_docs_dim384() {
        let idx = make_index(384);
        for i in 0..100u64 {
            let v = random_vec(384, i);
            idx.add(
                &v,
                DocEntry {
                    id: format!("doc_{i}"),
                    text: format!("document {i}"),
                    metadata: serde_json::json!({"index": i}),
                },
            );
        }
        let query = random_vec(384, 9999);
        let results = idx.search(&query, 5);
        assert_eq!(results.len(), 5);
        // Scores must be descending.
        for w in results.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }
}
