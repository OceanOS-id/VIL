use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use parking_lot::RwLock;
use serde::Serialize;
use vil_llm::{ChatMessage, LlmProvider};
use vil_log::app_log;
use vil_macros::VilAiEvent;

use crate::metrics::VariantMetrics;
use crate::policy::PromotionPolicy;
use crate::variant::ModelVariant;

/// Result of a single serve call.
#[derive(Debug, Clone, Serialize, VilAiEvent)]
pub struct ServeResult {
    pub content: String,
    pub variant_name: String,
    pub version: u32,
    pub latency_ms: u64,
}

/// Error type for serving operations.
#[derive(Debug)]
pub enum ServeError {
    NoVariants,
    AllVariantsZeroWeight,
    LlmError(String),
}

impl std::fmt::Display for ServeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoVariants => write!(f, "no model variants configured"),
            Self::AllVariantsZeroWeight => write!(f, "all variants have zero weight"),
            Self::LlmError(e) => write!(f, "LLM error: {}", e),
        }
    }
}
impl std::error::Error for ServeError {}

/// Differential model server — serves requests via weighted random variant
/// selection and tracks per-variant metrics.
pub struct ModelServer {
    variants: RwLock<Vec<ModelVariant>>,
    metrics: DashMap<String, VariantMetrics>,
    policy: PromotionPolicy,
    /// Simple counter for deterministic weighted round-robin (avoids needing
    /// rand crate).
    counter: std::sync::atomic::AtomicU64,
}

impl ModelServer {
    /// Create a new `ModelServer` with the given variants and policy.
    pub fn new(variants: Vec<ModelVariant>, policy: PromotionPolicy) -> Self {
        let metrics = DashMap::new();
        for v in &variants {
            metrics.insert(v.name.clone(), VariantMetrics::default());
        }
        Self {
            variants: RwLock::new(variants),
            metrics,
            policy,
            counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Serve a chat request — select a variant via weighted selection, call its
    /// provider, and record metrics.
    pub async fn serve(&self, messages: &[ChatMessage]) -> Result<ServeResult, ServeError> {
        let (name, version, provider) = self.select_variant()?;

        let start = Instant::now();
        let result = provider.chat(messages).await;
        let latency_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(response) => {
                if let Some(mut m) = self.metrics.get_mut(&name) {
                    m.record_request(latency_ms);
                }
                Ok(ServeResult {
                    content: response.content,
                    variant_name: name,
                    version,
                    latency_ms,
                })
            }
            Err(e) => {
                if let Some(mut m) = self.metrics.get_mut(&name) {
                    m.record_error();
                }
                Err(ServeError::LlmError(e.to_string()))
            }
        }
    }

    /// Record a quality score for a variant (e.g. from user feedback).
    pub fn record_quality(&self, variant_name: &str, score: f64) {
        if let Some(mut m) = self.metrics.get_mut(variant_name) {
            m.record_quality(score);
        }
    }

    /// Promote a variant to 100% traffic — set its weight to 1.0 and all
    /// others to 0.0.
    pub fn promote(&self, variant_name: &str) {
        let mut variants = self.variants.write();
        for v in variants.iter_mut() {
            if v.name == variant_name {
                v.weight = 1.0;
                app_log!(Info, "model_serving_promote", { variant: variant_name.to_string() });
            } else {
                v.weight = 0.0;
            }
        }
    }

    /// Roll back (remove) a variant. Traffic redistributes to remaining variants.
    pub fn rollback(&self, variant_name: &str) {
        let mut variants = self.variants.write();
        let before = variants.len();
        variants.retain(|v| v.name != variant_name);
        if variants.len() < before {
            app_log!(Warn, "model_serving_rollback", { variant: variant_name.to_string() });
            self.metrics.remove(variant_name);
        }
    }

    /// Get a snapshot of all variant metrics.
    pub fn get_metrics(&self) -> Vec<(String, VariantMetrics)> {
        self.metrics
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Apply promotion policy automatically across all variants.
    pub fn apply_policy(&self) {
        let variant_names: Vec<String> = {
            let variants = self.variants.read();
            variants.iter().map(|v| v.name.clone()).collect()
        };

        for name in &variant_names {
            if let Some(m) = self.metrics.get(name) {
                if self.policy.should_promote(&m) {
                    self.promote(name);
                    return;
                }
                if self.policy.should_rollback(&m) {
                    self.rollback(name);
                    return;
                }
            }
        }
    }

    /// Number of active variants.
    pub fn variant_count(&self) -> usize {
        self.variants.read().len()
    }

    /// Select a variant using deterministic weighted round-robin.
    fn select_variant(&self) -> Result<(String, u32, Arc<dyn LlmProvider>), ServeError> {
        let variants = self.variants.read();
        if variants.is_empty() {
            return Err(ServeError::NoVariants);
        }

        let total_weight: f32 = variants.iter().map(|v| v.weight).sum();
        if total_weight <= 0.0 {
            return Err(ServeError::AllVariantsZeroWeight);
        }

        // Deterministic weighted selection using an atomic counter
        let tick = self.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let point = (tick % 10000) as f32 / 10000.0 * total_weight;

        let mut cumulative = 0.0f32;
        for v in variants.iter() {
            cumulative += v.weight;
            if point < cumulative {
                return Ok((v.name.clone(), v.version, Arc::clone(&v.provider)));
            }
        }

        // Fallback to last variant
        let last = variants.last().unwrap();
        Ok((last.name.clone(), last.version, Arc::clone(&last.provider)))
    }
}
