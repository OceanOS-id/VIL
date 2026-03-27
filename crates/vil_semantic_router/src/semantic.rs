//! Semantic types for semantic router operations.
//!
//! VIL process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after a query is routed to a target.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct RouteEvent {
    pub query: String,
    pub route_name: String,
    pub target: String,
    pub confidence: f32,
    pub is_default: bool,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of semantic router failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RouteFaultType {
    NoRoutesConfigured,
    ClassificationFailed,
    InvalidRoute,
}

/// Emitted when a routing operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct RouteFault {
    pub error_type: RouteFaultType,
    pub message: String,
}

impl RouteFault {
    pub fn no_routes(msg: impl Into<String>) -> Self {
        Self { error_type: RouteFaultType::NoRoutesConfigured, message: msg.into() }
    }

    pub fn classification_failed(msg: impl Into<String>) -> Self {
        Self { error_type: RouteFaultType::ClassificationFailed, message: msg.into() }
    }

    pub fn invalid_route(msg: impl Into<String>) -> Self {
        Self { error_type: RouteFaultType::InvalidRoute, message: msg.into() }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks the current state of the semantic router.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct RouterState {
    pub total_queries: u64,
    pub total_default_fallbacks: u64,
    pub route_count: u64,
    pub avg_confidence: f32,
}
