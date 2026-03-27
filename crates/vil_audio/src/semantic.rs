//! Semantic types for audio transcription operations.
//!
//! These types follow VIL's process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after an audio transcription completes.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct AudioEvent {
    pub format: String,
    pub duration_ms: u64,
    pub language: String,
    pub segments: usize,
    pub latency_ms: u64,
    pub confidence: f32,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of audio processing failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AudioFaultType {
    UnsupportedFormat,
    EmptyAudio,
    ModelNotFound,
    TranscriptionFailed,
}

/// Emitted when an audio operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct AudioFault {
    pub error_type: AudioFaultType,
    pub message: String,
    pub format: Option<String>,
}

impl AudioFault {
    pub fn unsupported_format(fmt: &str) -> Self {
        Self {
            error_type: AudioFaultType::UnsupportedFormat,
            message: format!("unsupported format: {}", fmt),
            format: Some(fmt.into()),
        }
    }

    pub fn empty_audio() -> Self {
        Self {
            error_type: AudioFaultType::EmptyAudio,
            message: "audio data is empty".into(),
            format: None,
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks cumulative audio transcription statistics.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct AudioState {
    pub total_transcriptions: u64,
    pub total_duration_ms: u64,
    pub total_errors: u64,
    pub avg_confidence: f64,
    pub avg_latency_ms: f64,
}

impl AudioState {
    pub fn record(&mut self, event: &AudioEvent) {
        self.total_transcriptions += 1;
        self.total_duration_ms += event.duration_ms;
        let n = self.total_transcriptions as f64;
        self.avg_confidence =
            self.avg_confidence * (n - 1.0) / n + event.confidence as f64 / n;
        self.avg_latency_ms =
            self.avg_latency_ms * (n - 1.0) / n + event.latency_ms as f64 / n;
    }

    pub fn record_error(&mut self) {
        self.total_errors += 1;
    }
}
