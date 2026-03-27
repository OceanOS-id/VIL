//! Semantic types for image analysis operations.
//!
//! These types follow VIL's process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after an image analysis completes.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct VisionEvent {
    pub format: String,
    pub objects_detected: usize,
    pub ocr_extracted: bool,
    pub latency_ms: u64,
    pub model: String,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of vision processing failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VisionFaultType {
    UnsupportedFormat,
    EmptyImage,
    AnalysisFailed,
    ModelNotAvailable,
}

/// Emitted when a vision operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct VisionFault {
    pub error_type: VisionFaultType,
    pub message: String,
    pub model: Option<String>,
}

impl VisionFault {
    pub fn unsupported_format(fmt: &str) -> Self {
        Self {
            error_type: VisionFaultType::UnsupportedFormat,
            message: format!("unsupported format: {}", fmt),
            model: None,
        }
    }

    pub fn empty_image() -> Self {
        Self {
            error_type: VisionFaultType::EmptyImage,
            message: "image data is empty".into(),
            model: None,
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks cumulative vision analysis statistics.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct VisionState {
    pub total_analyses: u64,
    pub total_objects_detected: u64,
    pub total_ocr_extractions: u64,
    pub total_errors: u64,
    pub avg_latency_ms: f64,
}

impl VisionState {
    pub fn record(&mut self, event: &VisionEvent) {
        self.total_analyses += 1;
        self.total_objects_detected += event.objects_detected as u64;
        if event.ocr_extracted {
            self.total_ocr_extractions += 1;
        }
        let n = self.total_analyses as f64;
        self.avg_latency_ms =
            self.avg_latency_ms * (n - 1.0) / n + event.latency_ms as f64 / n;
    }

    pub fn record_error(&mut self) {
        self.total_errors += 1;
    }
}
