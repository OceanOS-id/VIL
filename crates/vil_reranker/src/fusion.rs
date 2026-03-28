use crate::reranker::{RerankCandidate, RerankError, RerankResult, Reranker};
use async_trait::async_trait;
use std::collections::HashMap;

/// Reciprocal Rank Fusion (RRF) reranker.
///
/// Combines multiple ranked lists into a single ranking using the formula:
///   `score(d) = sum_over_lists( 1 / (k + rank(d)) )`
///
/// The constant `k` (default 60) controls how much weight is given to lower ranks.
/// Higher `k` compresses rank differences.
///
/// Usage: pass multiple ranked lists as the `ranked_lists` field, then call
/// `rerank` (the `query` and `candidates` args are used to populate the output
/// text fields).
pub struct RRFReranker {
    /// The fusion constant (default 60).
    pub k: f32,
    /// Pre-computed ranked lists. Each inner vec is a list of candidate IDs
    /// in rank order (best first).
    pub ranked_lists: Vec<Vec<String>>,
}

impl RRFReranker {
    pub fn new(k: f32, ranked_lists: Vec<Vec<String>>) -> Self {
        Self { k, ranked_lists }
    }

    /// Create with default k=60.
    pub fn with_default_k(ranked_lists: Vec<Vec<String>>) -> Self {
        Self::new(60.0, ranked_lists)
    }
}

#[async_trait]
impl Reranker for RRFReranker {
    async fn rerank(
        &self,
        _query: &str,
        candidates: &[RerankCandidate],
        top_k: usize,
    ) -> Result<Vec<RerankResult>, RerankError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        // Build a lookup from candidate ID to text.
        let text_map: HashMap<&str, &str> = candidates
            .iter()
            .map(|c| (c.id.as_str(), c.text.as_str()))
            .collect();

        // Compute RRF scores.
        let mut scores: HashMap<String, f32> = HashMap::new();

        for list in &self.ranked_lists {
            for (rank, id) in list.iter().enumerate() {
                let rrf_score = 1.0 / (self.k + rank as f32 + 1.0);
                *scores.entry(id.clone()).or_insert(0.0) += rrf_score;
            }
        }

        let mut results: Vec<RerankResult> = scores
            .into_iter()
            .map(|(id, score)| {
                let text = text_map.get(id.as_str()).unwrap_or(&"").to_string();
                RerankResult {
                    id,
                    text,
                    score,
                    rank: 0,
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(top_k);

        for (i, r) in results.iter_mut().enumerate() {
            r.rank = i;
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_candidates() -> Vec<RerankCandidate> {
        vec![
            RerankCandidate {
                id: "a".into(),
                text: "Doc A".into(),
                initial_score: 0.0,
            },
            RerankCandidate {
                id: "b".into(),
                text: "Doc B".into(),
                initial_score: 0.0,
            },
            RerankCandidate {
                id: "c".into(),
                text: "Doc C".into(),
                initial_score: 0.0,
            },
        ]
    }

    #[tokio::test]
    async fn rrf_combines_ranks() {
        // List 1: a > b > c
        // List 2: c > a > b
        // With k=60:
        //   a: 1/61 + 1/62 = 0.01639 + 0.01613 = 0.03252
        //   b: 1/62 + 1/63 = 0.01613 + 0.01587 = 0.03200
        //   c: 1/63 + 1/61 = 0.01587 + 0.01639 = 0.03226
        // So: a > c > b
        let ranked_lists = vec![
            vec!["a".into(), "b".into(), "c".into()],
            vec!["c".into(), "a".into(), "b".into()],
        ];

        let reranker = RRFReranker::with_default_k(ranked_lists);
        let results = reranker
            .rerank("query", &make_candidates(), 10)
            .await
            .unwrap();

        assert_eq!(results[0].id, "a");
        assert_eq!(results[1].id, "c");
        assert_eq!(results[2].id, "b");
    }

    #[tokio::test]
    async fn rrf_top_k() {
        let ranked_lists = vec![vec!["a".into(), "b".into(), "c".into()]];
        let reranker = RRFReranker::with_default_k(ranked_lists);
        let results = reranker
            .rerank("query", &make_candidates(), 1)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn rrf_empty_candidates() {
        let ranked_lists = vec![vec!["a".into()]];
        let reranker = RRFReranker::with_default_k(ranked_lists);
        let results = reranker.rerank("query", &[], 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn rrf_score_ordering() {
        let ranked_lists = vec![vec!["a".into(), "b".into(), "c".into()]];
        let reranker = RRFReranker::with_default_k(ranked_lists);
        let results = reranker
            .rerank("query", &make_candidates(), 10)
            .await
            .unwrap();
        for w in results.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }
}
