use async_trait::async_trait;
use std::fmt;
use std::sync::Arc;

use vil_llm::EmbeddingProvider;

use crate::store::VectorStore;

/// A chunk retrieved by the retriever, with relevance score.
#[derive(Debug, Clone)]
pub struct RetrievedChunk {
    pub content: String,
    pub doc_id: String,
    pub score: f32,
    pub metadata: serde_json::Value,
}

/// Errors from retrieval operations.
#[derive(Debug)]
pub enum RetrieverError {
    EmbeddingFailed(String),
    StoreFailed(String),
}

impl fmt::Display for RetrieverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmbeddingFailed(e) => write!(f, "embedding failed: {}", e),
            Self::StoreFailed(e) => write!(f, "store search failed: {}", e),
        }
    }
}

impl std::error::Error for RetrieverError {}

/// Trait for retrieval strategies.
#[async_trait]
pub trait Retriever: Send + Sync {
    /// Retrieve the top-k most relevant chunks for the query.
    async fn retrieve(
        &self,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<RetrievedChunk>, RetrieverError>;
}

// ---------------------------------------------------------------------------
// DenseRetriever — embed query, then vector search
// ---------------------------------------------------------------------------

/// Dense retriever: embeds the query text and performs vector similarity search.
pub struct DenseRetriever {
    embedder: Arc<dyn EmbeddingProvider>,
    store: Arc<dyn VectorStore>,
}

impl DenseRetriever {
    pub fn new(embedder: Arc<dyn EmbeddingProvider>, store: Arc<dyn VectorStore>) -> Self {
        Self { embedder, store }
    }
}

#[async_trait]
impl Retriever for DenseRetriever {
    async fn retrieve(
        &self,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<RetrievedChunk>, RetrieverError> {
        // Embed the query
        let embeddings = self
            .embedder
            .embed(&[query.to_string()])
            .await
            .map_err(|e| RetrieverError::EmbeddingFailed(e.to_string()))?;

        let query_embedding = embeddings
            .into_iter()
            .next()
            .ok_or_else(|| RetrieverError::EmbeddingFailed("no embedding returned".into()))?;

        // Search the store
        let results = self
            .store
            .search(&query_embedding, top_k)
            .await
            .map_err(|e| RetrieverError::StoreFailed(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|r| RetrievedChunk {
                content: r.chunk.content,
                doc_id: r.chunk.doc_id,
                score: r.score,
                metadata: r.chunk.metadata,
            })
            .collect())
    }
}

// ---------------------------------------------------------------------------
// HybridRetriever — combines multiple retrievers (future: keyword + dense)
// ---------------------------------------------------------------------------

/// Hybrid retriever: runs multiple retrieval strategies and merges results.
/// Currently wraps a dense retriever; future versions will add BM25/keyword search.
pub struct HybridRetriever {
    dense: DenseRetriever,
}

impl HybridRetriever {
    pub fn new(embedder: Arc<dyn EmbeddingProvider>, store: Arc<dyn VectorStore>) -> Self {
        Self {
            dense: DenseRetriever::new(embedder, store),
        }
    }
}

#[async_trait]
impl Retriever for HybridRetriever {
    async fn retrieve(
        &self,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<RetrievedChunk>, RetrieverError> {
        // For now, delegate to dense retriever.
        // Future: combine with BM25/keyword retriever and re-rank.
        self.dense.retrieve(query, top_k).await
    }
}
