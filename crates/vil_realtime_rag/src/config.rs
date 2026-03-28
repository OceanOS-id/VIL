/// Configuration for the real-time RAG pipeline.
#[derive(Debug, Clone)]
pub struct RealtimeRagConfig {
    /// Embedding vector dimension.
    pub dimension: usize,
    /// Number of top results to return.
    pub top_k: usize,
    /// Maximum number of cached query embeddings.
    pub cache_size: usize,
    /// Template for formatting context. Use `{chunks}` as placeholder.
    pub context_template: String,
}

impl Default for RealtimeRagConfig {
    fn default() -> Self {
        Self {
            dimension: 384,
            top_k: 5,
            cache_size: 10_000,
            context_template:
                "Context:\n{chunks}\n\nAnswer the question based on the context above.".into(),
        }
    }
}
