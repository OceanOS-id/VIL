use std::fmt;
use std::sync::Arc;

use vil_llm::{ChatMessage, EmbeddingProvider, LlmProvider};

use crate::chunk::{ChunkerStrategy, EmbeddedChunk};
use crate::retriever::{DenseRetriever, Retriever};
use crate::store::VectorStore;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum RagError {
    ChunkingFailed(String),
    EmbeddingFailed(String),
    StoreFailed(String),
    RetrievalFailed(String),
    GenerationFailed(String),
}

impl fmt::Display for RagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChunkingFailed(e) => write!(f, "chunking failed: {}", e),
            Self::EmbeddingFailed(e) => write!(f, "embedding failed: {}", e),
            Self::StoreFailed(e) => write!(f, "store failed: {}", e),
            Self::RetrievalFailed(e) => write!(f, "retrieval failed: {}", e),
            Self::GenerationFailed(e) => write!(f, "generation failed: {}", e),
        }
    }
}

impl std::error::Error for RagError {}

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Result of ingesting a document.
pub struct IngestResult {
    pub doc_id: String,
    pub chunks_stored: usize,
}

/// A source reference in a query result.
pub struct Source {
    pub doc_id: String,
    pub content: String,
    pub score: f32,
}

/// Result of a RAG query.
pub struct QueryResult {
    pub answer: String,
    pub sources: Vec<Source>,
}

// ---------------------------------------------------------------------------
// RagPipeline
// ---------------------------------------------------------------------------

/// Orchestrates the full RAG pipeline: ingest (chunk -> embed -> store)
/// and query (embed -> retrieve -> generate).
pub struct RagPipeline {
    chunker: Arc<dyn ChunkerStrategy>,
    embedder: Arc<dyn EmbeddingProvider>,
    store: Arc<dyn VectorStore>,
    retriever: Arc<dyn Retriever>,
    llm: Arc<dyn LlmProvider>,
}

impl RagPipeline {
    /// Create a new builder.
    pub fn builder() -> RagPipelineBuilder {
        RagPipelineBuilder::default()
    }

    /// Ingest a document: chunk -> embed -> store.
    pub async fn ingest(&self, doc_id: &str, content: &str) -> Result<IngestResult, RagError> {
        let chunks = self.chunker.chunk(doc_id, content);
        if chunks.is_empty() {
            return Ok(IngestResult {
                doc_id: doc_id.into(),
                chunks_stored: 0,
            });
        }

        let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let embeddings = self
            .embedder
            .embed(&texts)
            .await
            .map_err(|e| RagError::EmbeddingFailed(e.to_string()))?;

        let embedded: Vec<EmbeddedChunk> = chunks
            .into_iter()
            .zip(embeddings)
            .map(|(chunk, embedding)| EmbeddedChunk { chunk, embedding })
            .collect();

        let count = embedded.len();
        self.store
            .upsert(&embedded)
            .await
            .map_err(|e| RagError::StoreFailed(e.to_string()))?;

        Ok(IngestResult {
            doc_id: doc_id.into(),
            chunks_stored: count,
        })
    }

    /// Query: embed question -> retrieve context -> generate answer with LLM.
    pub async fn query(&self, question: &str) -> Result<QueryResult, RagError> {
        let retrieved = self
            .retriever
            .retrieve(question, 5)
            .await
            .map_err(|e| RagError::RetrievalFailed(e.to_string()))?;

        let context = retrieved
            .iter()
            .map(|r| r.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        let messages = vec![
            ChatMessage::system(format!(
                "You are a helpful assistant. Answer the question based on the following context.\n\nContext:\n{}",
                context
            )),
            ChatMessage::user(question),
        ];

        let response = self
            .llm
            .chat(&messages)
            .await
            .map_err(|e| RagError::GenerationFailed(e.to_string()))?;

        Ok(QueryResult {
            answer: response.content,
            sources: retrieved
                .iter()
                .map(|r| Source {
                    doc_id: r.doc_id.clone(),
                    content: r.content.clone(),
                    score: r.score,
                })
                .collect(),
        })
    }

    /// Get a reference to the retriever (for sharing with other plugins).
    pub fn retriever(&self) -> Arc<dyn Retriever> {
        self.retriever.clone()
    }

    /// Get a reference to the store.
    pub fn store(&self) -> Arc<dyn VectorStore> {
        self.store.clone()
    }
}

// ---------------------------------------------------------------------------
// RagPipelineBuilder
// ---------------------------------------------------------------------------

/// Builder for RagPipeline.
#[derive(Default)]
pub struct RagPipelineBuilder {
    chunker: Option<Arc<dyn ChunkerStrategy>>,
    embedder: Option<Arc<dyn EmbeddingProvider>>,
    store: Option<Arc<dyn VectorStore>>,
    retriever: Option<Arc<dyn Retriever>>,
    llm: Option<Arc<dyn LlmProvider>>,
}

impl RagPipelineBuilder {
    pub fn chunker(mut self, chunker: Arc<dyn ChunkerStrategy>) -> Self {
        self.chunker = Some(chunker);
        self
    }

    pub fn embedder(mut self, embedder: Arc<dyn EmbeddingProvider>) -> Self {
        self.embedder = Some(embedder);
        self
    }

    pub fn store(mut self, store: Arc<dyn VectorStore>) -> Self {
        self.store = Some(store);
        self
    }

    pub fn retriever(mut self, retriever: Arc<dyn Retriever>) -> Self {
        self.retriever = Some(retriever);
        self
    }

    pub fn llm(mut self, llm: Arc<dyn LlmProvider>) -> Self {
        self.llm = Some(llm);
        self
    }

    /// Build the pipeline. If no retriever is set, a DenseRetriever is created
    /// from the embedder and store.
    pub fn build(self) -> RagPipeline {
        let embedder = self.embedder.expect("RagPipeline requires an embedder");
        let store = self.store.expect("RagPipeline requires a store");
        let retriever = self
            .retriever
            .unwrap_or_else(|| Arc::new(DenseRetriever::new(embedder.clone(), store.clone())));

        RagPipeline {
            chunker: self.chunker.expect("RagPipeline requires a chunker"),
            embedder,
            store,
            retriever,
            llm: self.llm.expect("RagPipeline requires an LLM provider"),
        }
    }
}
