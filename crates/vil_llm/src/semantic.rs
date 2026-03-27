//! Semantic types for LLM operations.
//!
//! These types follow VIL's process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)
//!
//! Each type uses Tier B AI semantic derive macros so the VIL runtime
//! can route them to the correct tri-lane automatically.

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after every LLM chat completion.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct LlmResponseEvent {
    pub provider: String,
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub latency_ms: u64,
    pub finish_reason: String,
    pub cached: bool,
}

/// Emitted for each streaming token chunk.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct LlmStreamChunkEvent {
    pub provider: String,
    pub model: String,
    pub chunk_index: u32,
    pub token_count: u32,
    pub is_final: bool,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of LLM failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LlmFaultType {
    RateLimited,
    AuthenticationFailed,
    ModelNotFound,
    Timeout,
    ServerError,
    InvalidResponse,
}

/// Emitted when an LLM call fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct LlmFault {
    pub provider: String,
    pub model: String,
    pub error_type: LlmFaultType,
    pub message: String,
    pub retry_after_ms: Option<u64>,
}

impl LlmFault {
    /// Convenience constructor for rate-limit faults.
    pub fn rate_limited(provider: &str, model: &str, retry_ms: Option<u64>) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            error_type: LlmFaultType::RateLimited,
            message: "rate limited".into(),
            retry_after_ms: retry_ms,
        }
    }

    /// Convenience constructor for timeout faults.
    pub fn timeout(provider: &str, model: &str) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            error_type: LlmFaultType::Timeout,
            message: "request timeout".into(),
            retry_after_ms: None,
        }
    }

    /// Convenience constructor for authentication failures.
    pub fn auth_failed(provider: &str, model: &str) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            error_type: LlmFaultType::AuthenticationFailed,
            message: "authentication failed".into(),
            retry_after_ms: None,
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks cumulative LLM usage per provider.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct LlmUsageState {
    pub provider: String,
    pub total_requests: u64,
    pub total_tokens: u64,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub total_errors: u64,
    pub avg_latency_ms: f64,
}

impl LlmUsageState {
    /// Update running statistics from a completion event.
    pub fn record(&mut self, event: &LlmResponseEvent) {
        self.total_requests += 1;
        self.total_tokens += event.total_tokens as u64;
        self.total_prompt_tokens += event.prompt_tokens as u64;
        self.total_completion_tokens += event.completion_tokens as u64;
        let n = self.total_requests as f64;
        self.avg_latency_ms =
            self.avg_latency_ms * (n - 1.0) / n + event.latency_ms as f64 / n;
    }

    /// Record an error occurrence.
    pub fn record_error(&mut self) {
        self.total_errors += 1;
    }
}
