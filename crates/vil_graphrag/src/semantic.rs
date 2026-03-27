//! Semantic types for graph-enhanced RAG operations.
//!
//! These types follow VIL's process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after a graph-enhanced RAG query completes.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct GraphRagEvent {
    pub query: String,
    pub entities_found: usize,
    pub relations_found: usize,
    pub latency_ms: u64,
    pub context_length: usize,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of GraphRAG failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GraphRagFaultType {
    ExtractionFailed,
    GraphBuildError,
    QueryFailed,
    EmptyGraph,
}

/// Emitted when a GraphRAG operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct GraphRagFault {
    pub operation: String,
    pub error_type: GraphRagFaultType,
    pub message: String,
}

impl GraphRagFault {
    pub fn extraction_failed(msg: &str) -> Self {
        Self {
            operation: "extract".into(),
            error_type: GraphRagFaultType::ExtractionFailed,
            message: msg.into(),
        }
    }

    pub fn query_failed(query: &str, msg: &str) -> Self {
        Self {
            operation: format!("query:{}", query),
            error_type: GraphRagFaultType::QueryFailed,
            message: msg.into(),
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks cumulative GraphRAG statistics.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct GraphRagState {
    pub total_queries: u64,
    pub total_entities: u64,
    pub total_relations: u64,
    pub total_errors: u64,
    pub avg_latency_ms: f64,
}

impl GraphRagState {
    pub fn record(&mut self, event: &GraphRagEvent) {
        self.total_queries += 1;
        self.total_entities += event.entities_found as u64;
        self.total_relations += event.relations_found as u64;
        let n = self.total_queries as f64;
        self.avg_latency_ms =
            self.avg_latency_ms * (n - 1.0) / n + event.latency_ms as f64 / n;
    }

    pub fn record_error(&mut self) {
        self.total_errors += 1;
    }
}
