// ── N05: Multimodal Search ──────────────────────────────────────────
use crate::modality::{Modality, MultimodalEmbedding};
use parking_lot::RwLock;

/// A search hit with distance/similarity score.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub index: usize,
    pub modality: Modality,
    pub distance: f32,
}

/// Multimodal search index — brute-force cosine similarity across modalities.
pub struct MultimodalSearch {
    index: RwLock<Vec<MultimodalEmbedding>>,
}

impl MultimodalSearch {
    pub fn new() -> Self {
        Self {
            index: RwLock::new(Vec::new()),
        }
    }

    /// Add an embedding to the index.
    pub fn add(&self, embedding: MultimodalEmbedding) {
        self.index.write().push(embedding);
    }

    /// Number of embeddings in the index.
    pub fn len(&self) -> usize {
        self.index.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.read().is_empty()
    }

    /// Search for the top-k nearest embeddings to a query.
    /// Optionally filter by modality.
    pub fn search(
        &self,
        query: &[f32],
        k: usize,
        modality_filter: Option<Modality>,
    ) -> Vec<SearchResult> {
        let idx = self.index.read();
        let mut scored: Vec<SearchResult> = idx
            .iter()
            .enumerate()
            .filter(|(_, emb)| {
                modality_filter.map_or(true, |m| emb.modality == m)
            })
            .filter(|(_, emb)| emb.dim() == query.len())
            .map(|(i, emb)| SearchResult {
                index: i,
                modality: emb.modality,
                distance: cosine_distance(query, &emb.embedding),
            })
            .collect();

        scored.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        scored
    }
}

impl Default for MultimodalSearch {
    fn default() -> Self {
        Self::new()
    }
}

fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a < f32::EPSILON || norm_b < f32::EPSILON {
        return 1.0; // max distance
    }
    1.0 - (dot / (norm_a * norm_b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_finds_nearest() {
        let search = MultimodalSearch::new();
        search.add(MultimodalEmbedding::new(Modality::Text, vec![1.0, 0.0, 0.0]));
        search.add(MultimodalEmbedding::new(Modality::Image, vec![0.0, 1.0, 0.0]));
        search.add(MultimodalEmbedding::new(Modality::Text, vec![0.9, 0.1, 0.0]));

        let results = search.search(&[1.0, 0.0, 0.0], 2, None);
        assert_eq!(results.len(), 2);
        // First result should be the exact match (index 0)
        assert_eq!(results[0].index, 0);
    }

    #[test]
    fn search_filter_by_modality() {
        let search = MultimodalSearch::new();
        search.add(MultimodalEmbedding::new(Modality::Text, vec![1.0, 0.0]));
        search.add(MultimodalEmbedding::new(Modality::Image, vec![0.9, 0.1]));
        search.add(MultimodalEmbedding::new(Modality::Text, vec![0.8, 0.2]));

        let results = search.search(&[1.0, 0.0], 10, Some(Modality::Image));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].modality, Modality::Image);
    }

    #[test]
    fn search_across_modalities() {
        let search = MultimodalSearch::new();
        search.add(MultimodalEmbedding::new(Modality::Text, vec![1.0, 0.0]));
        search.add(MultimodalEmbedding::new(Modality::Audio, vec![0.0, 1.0]));
        search.add(MultimodalEmbedding::new(Modality::Video, vec![0.7, 0.7]));

        let results = search.search(&[0.7, 0.7], 3, None);
        assert_eq!(results.len(), 3);
        // Video embedding should be closest to query
        assert_eq!(results[0].modality, Modality::Video);
    }

    #[test]
    fn search_empty_index() {
        let search = MultimodalSearch::new();
        let results = search.search(&[1.0, 0.0], 5, None);
        assert!(results.is_empty());
    }

    #[test]
    fn single_modality_search() {
        let search = MultimodalSearch::new();
        search.add(MultimodalEmbedding::new(Modality::Text, vec![1.0, 0.0]));
        let results = search.search(&[1.0, 0.0], 1, None);
        assert_eq!(results.len(), 1);
        assert!(results[0].distance < 0.01);
    }
}
