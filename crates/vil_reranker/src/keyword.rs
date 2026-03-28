use crate::reranker::{RerankCandidate, RerankError, RerankResult, Reranker};
use async_trait::async_trait;

/// Keyword-based reranker that boosts candidates containing query terms.
///
/// Scoring: `initial_score + boost * (matched_keywords / total_keywords)`.
pub struct KeywordReranker {
    /// How much to boost the score for full keyword overlap (0.0..1.0 typical).
    pub boost: f32,
}

impl KeywordReranker {
    pub fn new(boost: f32) -> Self {
        Self { boost }
    }
}

#[async_trait]
impl Reranker for KeywordReranker {
    async fn rerank(
        &self,
        query: &str,
        candidates: &[RerankCandidate],
        top_k: usize,
    ) -> Result<Vec<RerankResult>, RerankError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        let query_lower = query.to_lowercase();
        let keywords: Vec<&str> = query_lower.split_whitespace().collect();
        let keyword_count = keywords.len().max(1) as f32;

        let mut scored: Vec<RerankResult> = candidates
            .iter()
            .map(|c| {
                let text_lower = c.text.to_lowercase();
                let matches = keywords
                    .iter()
                    .filter(|kw| text_lower.contains(*kw))
                    .count() as f32;
                let score = c.initial_score + self.boost * (matches / keyword_count);
                RerankResult {
                    id: c.id.clone(),
                    text: c.text.clone(),
                    score,
                    rank: 0,
                }
            })
            .collect();

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
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

    fn make_candidates() -> Vec<RerankCandidate> {
        vec![
            RerankCandidate {
                id: "a".into(),
                text: "The cat sat on the mat".into(),
                initial_score: 0.5,
            },
            RerankCandidate {
                id: "b".into(),
                text: "Dogs are great pets".into(),
                initial_score: 0.5,
            },
            RerankCandidate {
                id: "c".into(),
                text: "The cat and the dog played".into(),
                initial_score: 0.5,
            },
        ]
    }

    #[tokio::test]
    async fn boosts_matching_candidates() {
        let reranker = KeywordReranker::new(1.0);
        let results = reranker
            .rerank("cat mat", &make_candidates(), 10)
            .await
            .unwrap();
        // "a" matches both "cat" and "mat", should rank first.
        assert_eq!(results[0].id, "a");
    }

    #[tokio::test]
    async fn top_k_truncation() {
        let reranker = KeywordReranker::new(1.0);
        let results = reranker.rerank("cat", &make_candidates(), 1).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn empty_candidates() {
        let reranker = KeywordReranker::new(1.0);
        let results = reranker.rerank("cat", &[], 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn rank_ordering() {
        let reranker = KeywordReranker::new(1.0);
        let results = reranker
            .rerank("cat", &make_candidates(), 10)
            .await
            .unwrap();
        for (i, r) in results.iter().enumerate() {
            assert_eq!(r.rank, i);
        }
        // Scores should be descending.
        for w in results.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }
}
