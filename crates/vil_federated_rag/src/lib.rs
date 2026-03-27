//! # vil_federated_rag
//!
//! N07 — Federated RAG: query multiple retrieval sources in parallel, merge results
//! with score-based interleaving and deduplication.

pub mod config;
pub mod federation;
pub mod merger;
pub mod source;

pub use config::FederatedConfig;
pub use federation::FederatedRetriever;
pub use merger::{FederatedResult, ResultMerger};
pub use source::{RagSource, RagSourceError, SourceResult};

// VIL integration layer
pub mod vil_semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use plugin::FederatedRagPlugin;
pub use vil_semantic::{FederatedEvent, FederatedFault, FederatedState};

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Arc;

    /// Helper: a mock RAG source that returns canned results.
    struct MockSource {
        id: String,
        results: Vec<SourceResult>,
        should_fail: bool,
    }

    impl MockSource {
        fn new(id: &str, results: Vec<SourceResult>) -> Self {
            Self { id: id.to_string(), results, should_fail: false }
        }

        fn failing(id: &str) -> Self {
            Self { id: id.to_string(), results: vec![], should_fail: true }
        }
    }

    #[async_trait]
    impl RagSource for MockSource {
        fn source_id(&self) -> &str { &self.id }

        async fn retrieve(&self, _query: &str, top_k: usize) -> Result<Vec<SourceResult>, RagSourceError> {
            if self.should_fail {
                return Err(RagSourceError::Unavailable("mock failure".into()));
            }
            let mut r = self.results.clone();
            r.truncate(top_k);
            Ok(r)
        }
    }

    fn make_result(source: &str, text: &str, score: f32) -> SourceResult {
        SourceResult {
            source_id: source.to_string(),
            text: text.to_string(),
            score,
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_single_source() {
        let src = MockSource::new("s1", vec![
            make_result("s1", "hello world", 0.9),
        ]);
        let mut retriever = FederatedRetriever::new(FederatedConfig::default());
        retriever.add_source(Arc::new(src));

        let result = retriever.retrieve("query", 5).await;
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.sources_queried, 1);
    }

    #[tokio::test]
    async fn test_multiple_sources_merge() {
        let s1 = MockSource::new("s1", vec![make_result("s1", "doc about cats", 0.9)]);
        let s2 = MockSource::new("s2", vec![make_result("s2", "doc about dogs", 0.8)]);
        let mut retriever = FederatedRetriever::new(FederatedConfig::default());
        retriever.add_source(Arc::new(s1));
        retriever.add_source(Arc::new(s2));

        let result = retriever.retrieve("animals", 5).await;
        assert_eq!(result.results.len(), 2);
        assert_eq!(result.sources_queried, 2);
    }

    #[tokio::test]
    async fn test_dedup_similar() {
        let s1 = MockSource::new("s1", vec![make_result("s1", "the quick brown fox", 0.9)]);
        let s2 = MockSource::new("s2", vec![make_result("s2", "the quick brown fox", 0.8)]);
        let mut retriever = FederatedRetriever::new(FederatedConfig::default());
        retriever.add_source(Arc::new(s1));
        retriever.add_source(Arc::new(s2));

        let result = retriever.retrieve("fox", 5).await;
        // Identical text should be deduped.
        assert_eq!(result.results.len(), 1);
    }

    #[tokio::test]
    async fn test_score_ordering() {
        let s1 = MockSource::new("s1", vec![make_result("s1", "low score", 0.3)]);
        let s2 = MockSource::new("s2", vec![make_result("s2", "high score", 0.95)]);
        let mut retriever = FederatedRetriever::new(FederatedConfig::default());
        retriever.add_source(Arc::new(s1));
        retriever.add_source(Arc::new(s2));

        let result = retriever.retrieve("q", 5).await;
        assert_eq!(result.results[0].text, "high score");
        assert_eq!(result.results[1].text, "low score");
    }

    #[tokio::test]
    async fn test_empty_source() {
        let s1 = MockSource::new("s1", vec![]);
        let mut retriever = FederatedRetriever::new(FederatedConfig::default());
        retriever.add_source(Arc::new(s1));

        let result = retriever.retrieve("q", 5).await;
        assert!(result.results.is_empty());
    }

    #[tokio::test]
    async fn test_source_failure_tolerance() {
        let good = MockSource::new("good", vec![make_result("good", "result", 0.9)]);
        let bad = MockSource::failing("bad");
        let mut retriever = FederatedRetriever::new(FederatedConfig {
            tolerate_failures: true,
            ..Default::default()
        });
        retriever.add_source(Arc::new(good));
        retriever.add_source(Arc::new(bad));

        let result = retriever.retrieve("q", 5).await;
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.sources_queried, 2);
    }

    #[tokio::test]
    async fn test_no_sources() {
        let retriever = FederatedRetriever::new(FederatedConfig::default());
        let result = retriever.retrieve("q", 5).await;
        assert!(result.results.is_empty());
        assert_eq!(result.sources_queried, 0);
    }

    #[tokio::test]
    async fn test_max_results_truncation() {
        let results: Vec<SourceResult> = (0..30)
            .map(|i| make_result("s1", &format!("doc {i}"), 1.0 - i as f32 * 0.01))
            .collect();
        let s1 = MockSource::new("s1", results);
        let mut retriever = FederatedRetriever::new(FederatedConfig {
            max_results: 5,
            ..Default::default()
        });
        retriever.add_source(Arc::new(s1));

        let result = retriever.retrieve("q", 30).await;
        assert_eq!(result.results.len(), 5);
    }
}
