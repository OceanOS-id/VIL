//! Semantic types for SQL agent operations.
//!
//! These types follow VIL's process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after SQL generation from natural language completes.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct SqlAgentEvent {
    pub query_text: String,
    pub generated_sql: String,
    pub table_name: String,
    pub is_safe: bool,
    pub latency_ms: u64,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of SQL agent failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SqlAgentFaultType {
    TableNotFound,
    ColumnNotFound,
    InjectionDetected,
    GenerationFailed,
    ValidationFailed,
}

/// Emitted when a SQL agent operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct SqlAgentFault {
    pub error_type: SqlAgentFaultType,
    pub message: String,
    pub query_text: Option<String>,
}

impl SqlAgentFault {
    pub fn table_not_found(table: &str) -> Self {
        Self {
            error_type: SqlAgentFaultType::TableNotFound,
            message: format!("table not found: {}", table),
            query_text: None,
        }
    }

    pub fn injection_detected(query: &str) -> Self {
        Self {
            error_type: SqlAgentFaultType::InjectionDetected,
            message: "potential SQL injection detected".into(),
            query_text: Some(query.into()),
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks cumulative SQL agent statistics.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct SqlAgentState {
    pub total_queries: u64,
    pub safe_queries: u64,
    pub unsafe_queries: u64,
    pub total_errors: u64,
    pub avg_latency_ms: f64,
}

impl SqlAgentState {
    pub fn record(&mut self, event: &SqlAgentEvent) {
        self.total_queries += 1;
        if event.is_safe {
            self.safe_queries += 1;
        } else {
            self.unsafe_queries += 1;
        }
        let n = self.total_queries as f64;
        self.avg_latency_ms =
            self.avg_latency_ms * (n - 1.0) / n + event.latency_ms as f64 / n;
    }

    pub fn record_error(&mut self) {
        self.total_errors += 1;
    }
}
