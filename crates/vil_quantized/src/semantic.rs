//! Semantic types for Quantized Model Runtime operations.
//!
//! These types follow VIL's process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after every quantized model inference.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct QuantizeEvent {
    pub model_path: String,
    pub format: String,
    pub prompt_length: usize,
    pub max_tokens: usize,
    pub memory_mb: f64,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of quantized runtime failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QuantizeFaultType {
    ModelNotLoaded,
    ModelLoadFailed,
    InferenceFailed,
    OutOfMemory,
    InternalError,
}

/// Emitted when a quantized runtime operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct QuantizeFault {
    pub error_type: QuantizeFaultType,
    pub message: String,
    pub model_path: Option<String>,
}

impl QuantizeFault {
    pub fn not_loaded(path: &str) -> Self {
        Self {
            error_type: QuantizeFaultType::ModelNotLoaded,
            message: "model not loaded — call load() first".into(),
            model_path: Some(path.into()),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            error_type: QuantizeFaultType::InternalError,
            message: msg.into(),
            model_path: None,
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks cumulative quantized inference statistics.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct QuantizedState {
    pub total_inferences: u64,
    pub total_errors: u64,
    pub total_tokens_generated: u64,
    pub model_loaded: bool,
    pub model_path: String,
    pub format: String,
    pub memory_mb: f64,
}

impl QuantizedState {
    pub fn record(&mut self, event: &QuantizeEvent) {
        self.total_inferences += 1;
        self.total_tokens_generated += event.max_tokens as u64;
        self.model_path = event.model_path.clone();
        self.format = event.format.clone();
        self.memory_mb = event.memory_mb;
    }

    pub fn record_error(&mut self) {
        self.total_errors += 1;
    }
}
