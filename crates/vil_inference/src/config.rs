// =============================================================================
// VIL Inference — Configuration
// =============================================================================

use serde::{Deserialize, Serialize};

/// Configuration for a single model in the inference server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Unique model name / identifier.
    pub name: String,
    /// Number of pre-warmed model instances in the pool.
    pub pool_size: usize,
    /// Maximum batch size before the batcher flushes.
    pub max_batch_size: usize,
    /// Maximum wait time (ms) before flushing a partial batch.
    pub max_wait_ms: u64,
    /// Per-request timeout (ms).
    pub timeout_ms: u64,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            name: "default".into(),
            pool_size: 2,
            max_batch_size: 8,
            max_wait_ms: 10,
            timeout_ms: 5000,
        }
    }
}

/// Top-level inference server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    /// List of model configurations.
    pub models: Vec<ModelConfig>,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self { models: vec![] }
    }
}
