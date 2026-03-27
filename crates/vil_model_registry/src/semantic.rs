//! Semantic types for model registry operations.
//!
//! These types follow VIL's process-oriented semantic model:
//! - Events: immutable audit records (Data Lane)
//! - Faults: error signals (Control Lane)
//! - State: mutable tracked state (Data Lane)

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

// ── Events (Data Lane, immutable audit) ─────────────────────────────

/// Emitted after a model registry operation (register, promote, rollback).
#[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
pub struct RegistryEvent {
    pub model_name: String,
    pub version: u32,
    pub operation: String,
    pub provider: String,
}

// ── Faults (Control Lane, error signals) ────────────────────────────

/// Classification of registry failure modes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RegistryFaultType {
    ModelNotFound,
    VersionNotFound,
    PromotionFailed,
    RollbackFailed,
    DeprecationFailed,
}

/// Emitted when a registry operation fails.
#[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
pub struct RegistryFault {
    pub error_type: RegistryFaultType,
    pub message: String,
    pub model_name: Option<String>,
    pub version: Option<u32>,
}

impl RegistryFault {
    pub fn model_not_found(name: &str) -> Self {
        Self {
            error_type: RegistryFaultType::ModelNotFound,
            message: format!("model not found: {}", name),
            model_name: Some(name.into()),
            version: None,
        }
    }

    pub fn version_not_found(name: &str, version: u32) -> Self {
        Self {
            error_type: RegistryFaultType::VersionNotFound,
            message: format!("version {} not found for model {}", version, name),
            model_name: Some(name.into()),
            version: Some(version),
        }
    }
}

// ── State (Data Lane, mutable tracked) ──────────────────────────────

/// Tracks cumulative model registry statistics.
#[derive(Clone, Debug, Default, Serialize, Deserialize, VilAiState)]
pub struct RegistryState {
    pub total_models: u64,
    pub total_versions: u64,
    pub total_promotions: u64,
    pub total_rollbacks: u64,
    pub total_errors: u64,
}

impl RegistryState {
    pub fn record(&mut self, event: &RegistryEvent) {
        match event.operation.as_str() {
            "register" => self.total_versions += 1,
            "promote" => self.total_promotions += 1,
            "rollback" => self.total_rollbacks += 1,
            _ => {}
        }
    }

    pub fn record_error(&mut self) {
        self.total_errors += 1;
    }
}
