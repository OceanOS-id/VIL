use async_trait::async_trait;
use std::sync::Arc;
use vil_embedder::EmbedProvider;
use vil_embedder::similarity::cosine_similarity;
use crate::reranker::{RerankCandidate, RerankError, RerankResult, Reranker};

/// Simulated cross-encoder reranker.
///
/// Embeds the query and each candidate document using an [`EmbedProvider`],
/// then scores by cosine similarity. This approximates a true cross-encoder
/// using a bi-encoder; swap in a real cross-encoder model for production.
pub struct CrossEncoderReranker {
    embedder: Arc<dyn EmbedProvider>,
}

impl CrossEncoderReranker {
    pub fn new(embedder: Arc<dyn EmbedProvider>) -> Self {
        Self { embedder }
    }
}

#[async_trait]
impl Reranker for CrossEncoderReranker {
    async fn rerank(
        &self,
        query: &str,
        candidates: &[RerankCandidate],
        top_k: usize,
    ) -> Result<Vec<RerankResult>, RerankError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        // Embed query.
        let query_vec = self
            .embedder
            .embed_one(query)
            .await
            .map_err(|e| RerankError::ProviderError(format!("{e}")))?;

        // Embed all candidate texts.
        let texts: Vec<String> = candidates.iter().map(|c| c.text.clone()).collect();
        let doc_vecs = self
            .embedder
            .embed_batch(&texts)
            .await
            .map_err(|e| RerankError::ProviderError(format!("{e}")))?;

        // Score by cosine similarity.
        let mut scored: Vec<RerankResult> = candidates
            .iter()
            .zip(doc_vecs.iter())
            .map(|(c, dv)| {
                let score = cosine_similarity(&query_vec, dv);
                RerankResult {
                    id: c.id.clone(),
                    text: c.text.clone(),
                    score,
                    rank: 0,
                }
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        for (i, r) in scored.iter_mut().enumerate() {
            r.rank = i;
        }

        Ok(scored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vil_embedder::EmbedError;

    /// Dummy embedder that returns fixed vectors for testing.
    struct DummyEmbedder;

    #[async_trait]
    impl EmbedProvider for DummyEmbedder {
        async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
            Ok(texts
                .iter()
                .map(|t| {
                    // Simple hash-based embedding for deterministic tests.
                    let hash = t.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
                    vec![
                        (hash % 100) as f32 / 100.0,
                        ((hash >> 8) % 100) as f32 / 100.0,
                        ((hash >> 16) % 100) as f32 / 100.0,
                    ]
                })
                .collect())
        }

        fn dimension(&self) -> usize {
            3
        }

        fn model_name(&self) -> &str {
            "example"
        }
    }

    #[tokio::test]
    async fn cross_encoder_ranks_by_similarity() {
        let embedder: Arc<dyn EmbedProvider> = Arc::new(DummyEmbedder);
        let reranker = CrossEncoderReranker::new(embedder);

        let candidates = vec![
            RerankCandidate { id: "a".into(), text: "hello world".into(), initial_score: 0.5 },
            RerankCandidate { id: "b".into(), text: "goodbye moon".into(), initial_score: 0.5 },
        ];

        let results = reranker.rerank("hello world", &candidates, 10).await.unwrap();
        assert_eq!(results.len(), 2);
        // The candidate with text identical to the query should rank higher.
        assert_eq!(results[0].id, "a");
    }

    #[tokio::test]
    async fn cross_encoder_empty() {
        let embedder: Arc<dyn EmbedProvider> = Arc::new(DummyEmbedder);
        let reranker = CrossEncoderReranker::new(embedder);
        let results = reranker.rerank("query", &[], 5).await.unwrap();
        assert!(results.is_empty());
    }
}
