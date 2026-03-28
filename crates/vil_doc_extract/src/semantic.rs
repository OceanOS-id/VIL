//! Semantic types for document extraction operations.
//!
//! These types follow VIL's process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after a document extraction completes.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct ExtractEvent {
    pub fields_extracted: usize,
    pub missing_required: usize,
    pub confidence: f32,
    pub is_complete: bool,
    pub latency_ms: u64,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of extraction failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExtractFaultType {
    NoFieldsExtracted,
    MissingRequiredFields,
    PatternError,
    EmptyInput,
}

/// Emitted when an extraction operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct ExtractFault {
    pub error_type: ExtractFaultType,
    pub message: String,
    pub missing_fields: Vec<String>,
}

impl ExtractFault {
    pub fn empty_input() -> Self {
        Self {
            error_type: ExtractFaultType::EmptyInput,
            message: "input text is empty".into(),
            missing_fields: Vec::new(),
        }
    }

    pub fn missing_required(fields: Vec<String>) -> Self {
        Self {
            error_type: ExtractFaultType::MissingRequiredFields,
            message: format!("missing required fields: {}", fields.join(", ")),
            missing_fields: fields,
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks cumulative extraction statistics.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct ExtractState {
    pub total_extractions: u64,
    pub complete_extractions: u64,
    pub total_fields_extracted: u64,
    pub total_errors: u64,
    pub avg_confidence: f64,
}

impl ExtractState {
    pub fn record(&mut self, event: &ExtractEvent) {
        self.total_extractions += 1;
        self.total_fields_extracted += event.fields_extracted as u64;
        if event.is_complete {
            self.complete_extractions += 1;
        }
        let n = self.total_extractions as f64;
        self.avg_confidence = self.avg_confidence * (n - 1.0) / n + event.confidence as f64 / n;
    }

    pub fn record_error(&mut self) {
        self.total_errors += 1;
    }
}
