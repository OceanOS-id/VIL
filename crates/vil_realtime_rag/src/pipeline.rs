use std::time::Instant;

use serde::Serialize;

use crate::config::RealtimeRagConfig;
use crate::index::{DocEntry, RealtimeIndex, RealtimeResult};
use crate::query_cache::QueryCache;
use vil_macros::VilAiEvent;

/// Sub-millisecond RAG pipeline.
///
/// Designed for latency-critical applications where the entire
/// retrieve-and-format cycle must complete in <1ms.
///
/// Requires:
/// - Pre-computed embeddings (no API call during query)
/// - In-memory index (no disk/network I/O)
/// - Query cache (skip embedding for repeated queries)
pub struct RealtimeRagPipeline {
    index: RealtimeIndex,
    query_cache: QueryCache,
    top_k: usize,
    context_template: String,
}

/// Result of a real-time RAG query.
#[derive(Serialize, VilAiEvent)]
pub struct RealtimeQueryResult {
    /// Formatted context string ready for LLM consumption.
    pub context: String,
    /// Individual retrieval results.
    pub chunks: Vec<RealtimeResult>,
    /// Whether the query embedding was served from cache.
    pub from_cache: bool,
    /// Wall-clock search time in microseconds.
    pub search_time_us: u64,
}

impl RealtimeRagPipeline {
    /// Create a new pipeline from configuration.
    pub fn new(config: RealtimeRagConfig) -> Self {
        Self {
            index: RealtimeIndex::new(config.dimension),
            query_cache: QueryCache::new(config.cache_size),
            top_k: config.top_k,
            context_template: config.context_template,
        }
    }

    /// Add a document with a pre-computed embedding.
    pub fn add_document(
        &self,
        embedding: &[f32],
        doc_id: &str,
        text: &str,
        metadata: serde_json::Value,
    ) {
        self.index.add(
            embedding,
            DocEntry {
                id: doc_id.to_string(),
                text: text.to_string(),
                metadata,
            },
        );
    }

    /// Query with a pre-computed query embedding.
    ///
    /// This is the <1ms path -- no API calls, no embedding computation.
    pub fn query_with_embedding(&self, query_embedding: &[f32]) -> RealtimeQueryResult {
        let start = Instant::now();
        let chunks = self.index.search(query_embedding, self.top_k);
        let search_time_us = start.elapsed().as_micros() as u64;

        let context = self.format_context(&chunks);
        RealtimeQueryResult {
            context,
            chunks,
            from_cache: false,
            search_time_us,
        }
    }

    /// Query with cached embedding (cache hit = no embedding computation).
    ///
    /// If the query string is found in the cache, uses the cached embedding.
    /// Otherwise, uses `fallback_embedding` and stores it in the cache for
    /// future lookups.
    pub fn query_cached(
        &self,
        query: &str,
        fallback_embedding: &[f32],
    ) -> RealtimeQueryResult {
        let (embedding, from_cache) = if let Some(cached) = self.query_cache.get(query) {
            (cached, true)
        } else {
            self.query_cache.put(query, fallback_embedding.to_vec());
            (fallback_embedding.to_vec(), false)
        };

        let start = Instant::now();
        let chunks = self.index.search(&embedding, self.top_k);
        let search_time_us = start.elapsed().as_micros() as u64;

        let context = self.format_context(&chunks);
        RealtimeQueryResult {
            context,
            chunks,
            from_cache,
            search_time_us,
        }
    }

    /// Pre-warm the query cache with common queries and their embeddings.
    pub fn warm_cache(&self, queries: &[(String, Vec<f32>)]) {
        for (query, embedding) in queries {
            self.query_cache.put(query, embedding.clone());
        }
    }

    /// Number of documents in the index.
    pub fn doc_count(&self) -> usize {
        self.index.count()
    }

    /// Number of cached query embeddings.
    pub fn cache_size(&self) -> usize {
        self.query_cache.len()
    }

    /// Format retrieved chunks into a context string using the template.
    fn format_context(&self, chunks: &[RealtimeResult]) -> String {
        let chunks_text: String = chunks
            .iter()
            .enumerate()
            .map(|(i, r)| format!("[{}] {}", i + 1, r.text))
            .collect::<Vec<_>>()
            .join("\n");
        self.context_template.replace("{chunks}", &chunks_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RealtimeRagConfig;

    fn make_pipeline() -> RealtimeRagPipeline {
        RealtimeRagPipeline::new(RealtimeRagConfig {
            dimension: 4,
            top_k: 2,
            cache_size: 100,
            ..Default::default()
        })
    }

    #[test]
    fn add_document_and_count() {
        let p = make_pipeline();
        assert_eq!(p.doc_count(), 0);
        p.add_document(&[1.0, 0.0, 0.0, 0.0], "d1", "hello", serde_json::json!({}));
        assert_eq!(p.doc_count(), 1);
    }

    #[test]
    fn query_with_embedding_returns_results() {
        let p = make_pipeline();
        p.add_document(&[1.0, 0.0, 0.0, 0.0], "a", "first", serde_json::json!({}));
        p.add_document(&[0.0, 1.0, 0.0, 0.0], "b", "second", serde_json::json!({}));
        p.add_document(&[0.7, 0.7, 0.0, 0.0], "c", "third", serde_json::json!({}));

        let result = p.query_with_embedding(&[1.0, 0.0, 0.0, 0.0]);
        assert_eq!(result.chunks.len(), 2);
        assert_eq!(result.chunks[0].doc_id, "a");
        assert!(!result.from_cache);
    }

    #[test]
    fn context_formatting() {
        let p = RealtimeRagPipeline::new(RealtimeRagConfig {
            dimension: 2,
            top_k: 2,
            cache_size: 10,
            context_template: "CTX:\n{chunks}\nEND".into(),
        });
        p.add_document(&[1.0, 0.0], "x", "alpha", serde_json::json!({}));
        p.add_document(&[0.9, 0.1], "y", "beta", serde_json::json!({}));

        let result = p.query_with_embedding(&[1.0, 0.0]);
        assert!(result.context.contains("[1] alpha"));
        assert!(result.context.contains("[2] beta"));
        assert!(result.context.starts_with("CTX:"));
        assert!(result.context.ends_with("END"));
    }

    #[test]
    fn query_cached_stores_and_retrieves() {
        let p = make_pipeline();
        p.add_document(&[1.0, 0.0, 0.0, 0.0], "d", "doc", serde_json::json!({}));

        let emb = vec![1.0, 0.0, 0.0, 0.0];

        // First call: cache miss, stores embedding.
        let r1 = p.query_cached("test query", &emb);
        assert!(!r1.from_cache);
        assert_eq!(p.cache_size(), 1);

        // Second call: cache hit.
        let r2 = p.query_cached("test query", &emb);
        assert!(r2.from_cache);
    }

    #[test]
    fn warm_cache() {
        let p = make_pipeline();
        p.warm_cache(&[
            ("q1".into(), vec![1.0, 0.0, 0.0, 0.0]),
            ("q2".into(), vec![0.0, 1.0, 0.0, 0.0]),
        ]);
        assert_eq!(p.cache_size(), 2);
    }
}
