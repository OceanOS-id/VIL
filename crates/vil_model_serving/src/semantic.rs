// =============================================================================
// VIL Semantic Types — Model Serving
// =============================================================================

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

/// Events emitted by the Model Serving subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiEvent)]
pub enum ServingEvent {
    /// An inference request was served by a variant.
    InferenceServed {
        variant_name: String,
        version: u32,
        latency_ms: u64,
    },
    /// A variant was promoted to full traffic.
    VariantPromoted { variant_name: String },
    /// A variant was rolled back and removed.
    VariantRolledBack { variant_name: String },
    /// A quality score was recorded.
    QualityRecorded { variant_name: String, score: f64 },
    /// The auto-promote policy was applied.
    PolicyApplied { action: String },
}

/// Faults that can occur in the Model Serving subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiFault)]
pub enum ServingFault {
    /// No variants are configured.
    NoVariants,
    /// All variants have zero weight.
    AllVariantsZeroWeight,
    /// An LLM provider returned an error.
    LlmError { variant: String, reason: String },
}

/// Observable state of the Model Serving subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiState)]
pub struct ServingState {
    /// Number of active model variants.
    pub variant_count: usize,
    /// Per-variant metrics snapshot.
    pub variant_metrics: Vec<(String, VariantMetricsSummary)>,
}

/// Lightweight summary of per-variant metrics for state reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantMetricsSummary {
    pub requests: u64,
    pub errors: u64,
    pub avg_latency_ms: f64,
    pub avg_quality_score: f64,
}
