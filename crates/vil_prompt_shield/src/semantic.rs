//! Semantic types for Prompt Shield operations.
//!
//! These types follow VIL's process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after every prompt scan operation.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct ShieldEvent {
    pub input_length: usize,
    pub safe: bool,
    pub risk_score: f64,
    pub threat_count: usize,
    pub scan_time_us: u64,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of shield failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ShieldFaultType {
    PatternLoadFailed,
    ScanTimeout,
    ConfigInvalid,
    InternalError,
}

/// Emitted when a shield operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct ShieldFault {
    pub error_type: ShieldFaultType,
    pub message: String,
}

impl ShieldFault {
    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            error_type: ShieldFaultType::InternalError,
            message: msg.into(),
        }
    }

    pub fn config_invalid(msg: impl Into<String>) -> Self {
        Self {
            error_type: ShieldFaultType::ConfigInvalid,
            message: msg.into(),
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks cumulative shield scan statistics.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct ShieldState {
    pub total_scans: u64,
    pub total_blocked: u64,
    pub total_safe: u64,
    pub avg_scan_time_us: f64,
}

impl ShieldState {
    pub fn record(&mut self, event: &ShieldEvent) {
        self.total_scans += 1;
        if event.safe {
            self.total_safe += 1;
        } else {
            self.total_blocked += 1;
        }
        let n = self.total_scans as f64;
        self.avg_scan_time_us =
            self.avg_scan_time_us * (n - 1.0) / n + event.scan_time_us as f64 / n;
    }
}
