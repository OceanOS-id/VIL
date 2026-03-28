//! Federated retriever — query all sources in parallel, merge results.

use crate::config::FederatedConfig;
use crate::merger::{FederatedResult, ResultMerger};
use crate::source::{RagSource, SourceResult};
use std::sync::Arc;

/// Federated retriever that queries multiple RAG sources in parallel.
pub struct FederatedRetriever {
    pub sources: Vec<Arc<dyn RagSource>>,
    pub config: FederatedConfig,
    merger: ResultMerger,
}

impl FederatedRetriever {
    pub fn new(config: FederatedConfig) -> Self {
        let merger = ResultMerger::new(config.dedup_threshold);
        Self {
            sources: Vec::new(),
            config,
            merger,
        }
    }

    /// Add a source to the federation.
    pub fn add_source(&mut self, source: Arc<dyn RagSource>) {
        self.sources.push(source);
    }

    /// Query all sources in parallel and merge results.
    pub async fn retrieve(&self, query: &str, top_k: usize) -> FederatedResult {
        let start = std::time::Instant::now();
        let sources_queried = self.sources.len();

        let mut handles = Vec::new();
        for source in &self.sources {
            let source = Arc::clone(source);
            let query = query.to_string();
            handles.push(tokio::spawn(
                async move { source.retrieve(&query, top_k).await },
            ));
        }

        let mut result_sets: Vec<Vec<SourceResult>> = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(results)) => result_sets.push(results),
                Ok(Err(_)) if self.config.tolerate_failures => { /* skip failed source */ }
                Ok(Err(e)) => {
                    // If not tolerating failures, still just log and continue for now.
                    eprintln!("Source failed: {e}");
                }
                Err(_) if self.config.tolerate_failures => { /* task panic, skip */ }
                Err(e) => {
                    eprintln!("Task panic: {e}");
                }
            }
        }

        let mut results = self.merger.merge(result_sets);
        results.truncate(self.config.max_results);

        FederatedResult {
            results,
            sources_queried,
            total_ms: start.elapsed().as_millis() as u64,
        }
    }
}
