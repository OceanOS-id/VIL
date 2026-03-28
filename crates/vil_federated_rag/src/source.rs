//! RAG source trait and result types.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single result from a RAG source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceResult {
    pub source_id: String,
    pub text: String,
    pub score: f32,
    pub metadata: HashMap<String, String>,
}

/// Trait for a retrieval source that can be queried.
#[async_trait]
pub trait RagSource: Send + Sync {
    /// Unique identifier for this source.
    fn source_id(&self) -> &str;

    /// Retrieve top-k results for the given query.
    async fn retrieve(
        &self,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<SourceResult>, RagSourceError>;
}

/// Errors from RAG source retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RagSourceError {
    /// The source is unavailable (network, timeout, etc.).
    Unavailable(String),
    /// Query was malformed or rejected.
    InvalidQuery(String),
    /// Generic error.
    Other(String),
}

impl std::fmt::Display for RagSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(msg) => write!(f, "source unavailable: {msg}"),
            Self::InvalidQuery(msg) => write!(f, "invalid query: {msg}"),
            Self::Other(msg) => write!(f, "rag source error: {msg}"),
        }
    }
}

impl std::error::Error for RagSourceError {}
