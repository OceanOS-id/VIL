use crate::distance::DistanceMetric;

/// Configuration for the HNSW index.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HnswConfig {
    /// Maximum number of connections per node at each layer.
    pub m: usize,
    /// Beam width during index construction.
    pub ef_construction: usize,
    /// Beam width during search.
    pub ef_search: usize,
    /// Distance metric to use.
    pub metric: DistanceMetric,
}

impl Default for HnswConfig {
    fn default() -> Self {
        Self {
            m: 16,
            ef_construction: 200,
            ef_search: 50,
            metric: DistanceMetric::Cosine,
        }
    }
}
