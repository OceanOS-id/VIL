//! Semantic types for A/B testing operations.
//!
//! These types follow VIL's process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted when an experiment assignment or report is generated.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct AbTestEvent {
    pub experiment: String,
    pub variant_assigned: String,
    pub is_conversion: bool,
    pub total_impressions: u64,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of A/B test failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AbTestFaultType {
    ExperimentNotFound,
    VariantNotFound,
    InsufficientData,
    InvalidConfiguration,
}

/// Emitted when an A/B test operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct AbTestFault {
    pub error_type: AbTestFaultType,
    pub message: String,
    pub experiment: Option<String>,
}

impl AbTestFault {
    pub fn experiment_not_found(name: &str) -> Self {
        Self {
            error_type: AbTestFaultType::ExperimentNotFound,
            message: format!("experiment not found: {}", name),
            experiment: Some(name.into()),
        }
    }

    pub fn insufficient_data(experiment: &str) -> Self {
        Self {
            error_type: AbTestFaultType::InsufficientData,
            message: "insufficient data for statistical significance".into(),
            experiment: Some(experiment.into()),
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks cumulative A/B testing statistics.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct AbTestState {
    pub total_experiments: u64,
    pub total_impressions: u64,
    pub total_conversions: u64,
    pub significant_results: u64,
    pub total_errors: u64,
}

impl AbTestState {
    pub fn record(&mut self, event: &AbTestEvent) {
        self.total_impressions += 1;
        if event.is_conversion {
            self.total_conversions += 1;
        }
    }

    pub fn record_error(&mut self) {
        self.total_errors += 1;
    }
}
