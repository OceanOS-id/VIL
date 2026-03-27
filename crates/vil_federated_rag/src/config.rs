//! Configuration for federated RAG.

use serde::{Deserialize, Serialize};

/// Configuration for the federated retriever.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedConfig {
    /// Maximum time (ms) to wait for any single source.
    pub source_timeout_ms: u64,
    /// Deduplication similarity threshold (0.0–1.0).
    pub dedup_threshold: f32,
    /// Maximum total results to return.
    pub max_results: usize,
    /// Whether to tolerate individual source failures.
    pub tolerate_failures: bool,
}

impl Default for FederatedConfig {
    fn default() -> Self {
        Self {
            source_timeout_ms: 5000,
            dedup_threshold: 0.85,
            max_results: 20,
            tolerate_failures: true,
        }
    }
}
