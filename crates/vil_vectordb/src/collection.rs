use crate::config::HnswConfig;
use crate::hnsw::HnswIndex;
use crate::storage::{VectorRecord, VectorStorage};

/// A search result enriched with metadata from storage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub id: u64,
    pub score: f32,
    pub metadata: serde_json::Value,
    pub text: Option<String>,
}

/// A named collection that pairs an HNSW index with document storage.
pub struct Collection {
    name: String,
    index: HnswIndex,
    storage: VectorStorage,
    dimension: usize,
}

impl Collection {
    /// Create a new collection.
    pub fn new(name: &str, dimension: usize, config: HnswConfig) -> Self {
        Self {
            name: name.to_string(),
            index: HnswIndex::new(dimension, config),
            storage: VectorStorage::new(),
            dimension,
        }
    }

    /// Add a vector with optional metadata and text. Returns the assigned ID.
    pub fn add(
        &self,
        vector: Vec<f32>,
        metadata: serde_json::Value,
        text: Option<String>,
    ) -> u64 {
        let id = self.storage.next_id();
        let record = VectorRecord {
            id,
            vector: vector.clone(),
            metadata,
            text,
        };
        self.storage.insert(record);
        // Best-effort insert into index; dimension mismatch would be a bug here.
        self.index
            .insert(id, vector)
            .expect("vector dimension should match collection dimension");
        id
    }

    /// Search for the `top_k` most similar vectors.
    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<SearchResult> {
        let hits = self.index.search(query, top_k);
        hits.into_iter()
            .filter_map(|hit| {
                let record = self.storage.get(hit.id)?;
                Some(SearchResult {
                    id: hit.id,
                    score: hit.score,
                    metadata: record.metadata,
                    text: record.text,
                })
            })
            .collect()
    }

    /// Delete a vector by ID. Returns true if it existed.
    pub fn delete(&self, id: u64) -> bool {
        let idx_deleted = self.index.delete(id);
        let storage_deleted = self.storage.delete(id);
        idx_deleted || storage_deleted
    }

    /// Number of vectors in this collection.
    pub fn count(&self) -> usize {
        self.storage.count()
    }

    /// Collection name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Vector dimension.
    pub fn dimension(&self) -> usize {
        self.dimension
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_collection() -> Collection {
        Collection::new("test", 3, HnswConfig::default())
    }

    #[test]
    fn add_and_count() {
        let col = test_collection();
        let id = col.add(vec![1.0, 0.0, 0.0], serde_json::json!({}), None);
        assert_eq!(id, 1);
        assert_eq!(col.count(), 1);
    }

    #[test]
    fn search_returns_sorted_by_score() {
        let col = test_collection();
        col.add(vec![1.0, 0.0, 0.0], serde_json::json!({"label": "a"}), Some("doc a".into()));
        col.add(vec![0.9, 0.1, 0.0], serde_json::json!({"label": "b"}), Some("doc b".into()));
        col.add(vec![0.0, 1.0, 0.0], serde_json::json!({"label": "c"}), Some("doc c".into()));

        let results = col.search(&[1.0, 0.0, 0.0], 3);
        assert_eq!(results.len(), 3);
        // Results should be sorted by score descending (highest similarity first)
        assert!(results[0].score >= results[1].score);
        assert!(results[1].score >= results[2].score);
        assert_eq!(results[0].text, Some("doc a".into()));
    }

    #[test]
    fn delete_from_collection() {
        let col = test_collection();
        let id = col.add(vec![1.0, 0.0, 0.0], serde_json::json!({}), None);
        assert!(col.delete(id));
        assert_eq!(col.count(), 0);
    }

    #[test]
    fn name_and_dimension() {
        let col = test_collection();
        assert_eq!(col.name(), "test");
        assert_eq!(col.dimension(), 3);
    }
}
