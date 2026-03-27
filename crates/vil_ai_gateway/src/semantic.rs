// =============================================================================
// VIL Semantic Types — AI Gateway
// =============================================================================

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

/// Events emitted by the AI Gateway subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiEvent)]
pub enum GatewayEvent {
    /// A chat request was successfully routed and completed.
    ChatCompleted {
        model_used: String,
        latency_ms: u64,
        cost_usd: f64,
        attempts: u32,
    },
    /// A failover occurred during request processing.
    Failover {
        from_model: String,
        to_model: String,
        reason: String,
    },
    /// A circuit breaker state changed.
    CircuitBreakerTripped { model: String },
    /// Health status of a model changed.
    HealthChanged {
        model: String,
        new_status: String,
    },
}

/// Faults that can occur in the AI Gateway subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiFault)]
pub enum GatewayFault {
    /// All providers failed to handle the request.
    AllProvidersFailed { attempts: Vec<(String, String)> },
    /// No healthy providers are available.
    NoHealthyProviders,
    /// Budget was exceeded.
    BudgetExceeded { message: String },
    /// A request timed out.
    Timeout { model: String, timeout_ms: u64 },
}

/// Observable state of the AI Gateway subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiState)]
pub struct GatewayState {
    /// Total number of requests processed.
    pub total_requests: u64,
    /// Total number of successful requests.
    pub total_successes: u64,
    /// Total number of failed requests.
    pub total_failures: u64,
    /// Total number of failovers performed.
    pub total_failovers: u64,
    /// Current success rate.
    pub success_rate: f64,
}
