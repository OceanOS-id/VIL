//! Semantic types for cost tracking operations.
//!
//! These types follow VIL's process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after a cost record is logged.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct CostEvent {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of cost tracking failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CostFaultType {
    BudgetExceeded,
    PricingNotConfigured,
    TrackingError,
}

/// Emitted when a cost tracking operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct CostFault {
    pub error_type: CostFaultType,
    pub message: String,
    pub model: Option<String>,
    pub budget_limit_usd: Option<f64>,
}

impl CostFault {
    pub fn budget_exceeded(model: &str, limit: f64, current: f64) -> Self {
        Self {
            error_type: CostFaultType::BudgetExceeded,
            message: format!("budget exceeded: ${:.4} > ${:.4}", current, limit),
            model: Some(model.into()),
            budget_limit_usd: Some(limit),
        }
    }

    pub fn pricing_not_configured(model: &str) -> Self {
        Self {
            error_type: CostFaultType::PricingNotConfigured,
            message: format!("no pricing configured for model: {}", model),
            model: Some(model.into()),
            budget_limit_usd: None,
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks cumulative cost statistics.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct CostState {
    pub total_requests: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
    pub total_errors: u64,
}

impl CostState {
    pub fn record(&mut self, event: &CostEvent) {
        self.total_requests += 1;
        self.total_input_tokens += event.input_tokens;
        self.total_output_tokens += event.output_tokens;
        self.total_cost_usd += event.cost_usd;
    }

    pub fn record_error(&mut self) {
        self.total_errors += 1;
    }
}
