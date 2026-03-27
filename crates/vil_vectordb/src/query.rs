use crate::collection::{Collection, SearchResult};

/// Fluent query builder for searching a collection.
pub struct QueryBuilder<'a> {
    collection: &'a Collection,
    vector: Option<Vec<f32>>,
    top_k: usize,
    min_score: Option<f32>,
    filter_fn: Option<Box<dyn Fn(&serde_json::Value) -> bool>>,
}

impl<'a> QueryBuilder<'a> {
    /// Create a new query builder targeting the given collection.
    pub fn new(collection: &'a Collection) -> Self {
        Self {
            collection,
            vector: None,
            top_k: 10,
            min_score: None,
            filter_fn: None,
        }
    }

    /// Set the query vector.
    pub fn vector(mut self, v: Vec<f32>) -> Self {
        self.vector = Some(v);
        self
    }

    /// Set the maximum number of results.
    pub fn top_k(mut self, k: usize) -> Self {
        self.top_k = k;
        self
    }

    /// Set a minimum similarity score threshold.
    pub fn min_score(mut self, s: f32) -> Self {
        self.min_score = Some(s);
        self
    }

    /// Set a metadata filter function.
    pub fn filter(mut self, f: impl Fn(&serde_json::Value) -> bool + 'static) -> Self {
        self.filter_fn = Some(Box::new(f));
        self
    }

    /// Execute the query and return results.
    ///
    /// # Panics
    /// Panics if no query vector was set.
    pub fn execute(&self) -> Vec<SearchResult> {
        let vector = self
            .vector
            .as_ref()
            .expect("query vector must be set before executing");

        // Fetch more results than top_k to account for filtering
        let fetch_k = if self.filter_fn.is_some() || self.min_score.is_some() {
            self.top_k * 4
        } else {
            self.top_k
        };

        let mut results = self.collection.search(vector, fetch_k);

        // Apply min_score filter
        if let Some(min) = self.min_score {
            results.retain(|r| r.score >= min);
        }

        // Apply metadata filter
        if let Some(ref filter) = self.filter_fn {
            results.retain(|r| filter(&r.metadata));
        }

        results.truncate(self.top_k);
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HnswConfig;

    fn setup_collection() -> Collection {
        let col = Collection::new("qtest", 3, HnswConfig::default());
        col.add(vec![1.0, 0.0, 0.0], serde_json::json!({"category": "A"}), Some("alpha".into()));
        col.add(vec![0.9, 0.1, 0.0], serde_json::json!({"category": "B"}), Some("beta".into()));
        col.add(vec![0.0, 1.0, 0.0], serde_json::json!({"category": "A"}), Some("gamma".into()));
        col.add(vec![0.0, 0.0, 1.0], serde_json::json!({"category": "C"}), Some("delta".into()));
        col
    }

    #[test]
    fn basic_query() {
        let col = setup_collection();
        let results = QueryBuilder::new(&col)
            .vector(vec![1.0, 0.0, 0.0])
            .top_k(2)
            .execute();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn query_with_min_score() {
        let col = setup_collection();
        let results = QueryBuilder::new(&col)
            .vector(vec![1.0, 0.0, 0.0])
            .top_k(10)
            .min_score(0.9)
            .execute();
        // Only vectors very similar to [1,0,0] should pass
        for r in &results {
            assert!(r.score >= 0.9, "score {} should be >= 0.9", r.score);
        }
    }

    #[test]
    fn query_with_filter() {
        let col = setup_collection();
        let results = QueryBuilder::new(&col)
            .vector(vec![1.0, 0.0, 0.0])
            .top_k(10)
            .filter(|meta| meta["category"] == "A")
            .execute();
        for r in &results {
            assert_eq!(r.metadata["category"], "A");
        }
    }

    #[test]
    fn query_top_k_respected() {
        let col = setup_collection();
        let results = QueryBuilder::new(&col)
            .vector(vec![1.0, 0.0, 0.0])
            .top_k(1)
            .execute();
        assert_eq!(results.len(), 1);
    }
}
