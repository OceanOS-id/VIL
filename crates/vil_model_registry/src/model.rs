use serde::{Deserialize, Serialize};

/// Status of a model version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelStatus {
    Active,
    Staging,
    Deprecated,
    Archived,
}

/// A single version of a model in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub name: String,
    pub version: u32,
    pub provider: String,
    pub config: serde_json::Value,
    pub status: ModelStatus,
    /// Deployment timestamp (Unix epoch seconds).
    pub deployed_at: u64,
}
