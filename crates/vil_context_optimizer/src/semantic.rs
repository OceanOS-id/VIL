//! Semantic types for Context Optimizer operations.
//!
//! These types follow VIL's process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after every context optimization operation.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct OptimizeEvent {
    pub original_count: usize,
    pub final_count: usize,
    pub tokens_saved: usize,
    pub compression_ratio: f32,
    pub strategy: String,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of optimizer failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OptimizeFaultType {
    BudgetExceeded,
    EmptyInput,
    TokenizerError,
    InternalError,
}

/// Emitted when an optimization operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct OptimizeFault {
    pub error_type: OptimizeFaultType,
    pub message: String,
}

impl OptimizeFault {
    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            error_type: OptimizeFaultType::InternalError,
            message: msg.into(),
        }
    }

    pub fn empty_input() -> Self {
        Self {
            error_type: OptimizeFaultType::EmptyInput,
            message: "no chunks provided for optimization".into(),
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks cumulative optimizer statistics.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct OptimizerState {
    pub total_optimizations: u64,
    pub total_tokens_saved: u64,
    pub total_chunks_processed: u64,
    pub avg_compression_ratio: f64,
}

impl OptimizerState {
    pub fn record(&mut self, event: &OptimizeEvent) {
        self.total_optimizations += 1;
        self.total_tokens_saved += event.tokens_saved as u64;
        self.total_chunks_processed += event.original_count as u64;
        let n = self.total_optimizations as f64;
        self.avg_compression_ratio =
            self.avg_compression_ratio * (n - 1.0) / n + event.compression_ratio as f64 / n;
    }
}
