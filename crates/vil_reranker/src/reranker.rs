use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A candidate document to be reranked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankCandidate {
    /// Unique identifier for this candidate.
    pub id: String,
    /// The text content.
    pub text: String,
    /// Score from the initial retrieval stage.
    pub initial_score: f32,
}

/// A reranked result with final score and rank position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResult {
    /// Candidate identifier.
    pub id: String,
    /// The text content.
    pub text: String,
    /// Final reranking score.
    pub score: f32,
    /// 0-based rank position.
    pub rank: usize,
}

/// Errors that may occur during reranking.
#[derive(Debug, Clone)]
pub enum RerankError {
    /// The underlying embedder or LLM call failed.
    ProviderError(String),
    /// No candidates provided.
    EmptyCandidates,
}

impl fmt::Display for RerankError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RerankError::ProviderError(msg) => write!(f, "provider error: {msg}"),
            RerankError::EmptyCandidates => write!(f, "empty candidates"),
        }
    }
}

impl std::error::Error for RerankError {}

/// Trait for reranking strategies.
#[async_trait]
pub trait Reranker: Send + Sync {
    /// Rerank candidates for the given query, returning the top `top_k` results
    /// sorted by descending score.
    async fn rerank(
        &self,
        query: &str,
        candidates: &[RerankCandidate],
        top_k: usize,
    ) -> Result<Vec<RerankResult>, RerankError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rerank_error_display() {
        let e = RerankError::ProviderError("timeout".into());
        assert!(format!("{e}").contains("timeout"));

        let e = RerankError::EmptyCandidates;
        assert!(format!("{e}").contains("empty"));
    }

    #[test]
    fn candidate_serde_roundtrip() {
        let c = RerankCandidate {
            id: "doc1".into(),
            text: "hello".into(),
            initial_score: 0.9,
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: RerankCandidate = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "doc1");
    }

    #[test]
    fn result_serde_roundtrip() {
        let r = RerankResult {
            id: "doc1".into(),
            text: "hello".into(),
            score: 0.95,
            rank: 0,
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: RerankResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.rank, 0);
    }
}
