use std::sync::Arc;
use vil_llm::LlmProvider;

/// A model variant — one version of a model with an associated traffic weight.
pub struct ModelVariant {
    /// Human-readable name for this variant (e.g. "gpt4-v2", "claude-canary").
    pub name: String,
    /// The LLM provider backing this variant.
    pub provider: Arc<dyn LlmProvider>,
    /// Traffic weight (0.0 – 1.0). Weights across all variants are normalised
    /// at selection time so they don't need to sum to 1.0.
    pub weight: f32,
    /// Monotonic version identifier.
    pub version: u32,
}

impl ModelVariant {
    pub fn new(
        name: impl Into<String>,
        provider: Arc<dyn LlmProvider>,
        weight: f32,
        version: u32,
    ) -> Self {
        Self {
            name: name.into(),
            provider,
            weight,
            version,
        }
    }
}
