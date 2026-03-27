// =============================================================================
// VIL Semantic Types — Feature Store
// =============================================================================

use serde::{Deserialize, Serialize};
use vil_macros::{VilAiEvent, VilAiFault, VilAiState};

/// Events emitted by the Feature Store subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiEvent)]
pub enum FeatureEvent {
    /// A feature was written or updated.
    FeatureSet { key: String, version: u64 },
    /// A feature was retrieved.
    FeatureGet { key: String, found: bool },
    /// A batch retrieval was performed.
    BatchGet { keys: usize, found: usize },
    /// Expired entries were evicted.
    Eviction { evicted: usize, remaining: usize },
    /// A point-in-time snapshot was taken.
    SnapshotTaken { entries: usize, taken_at: u64 },
}

/// Faults that can occur in the Feature Store subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiFault)]
pub enum FeatureFault {
    /// An invalid key format was supplied.
    InvalidKey { raw: String },
    /// A TTL-related fault occurred.
    TtlOverflow { key: String, ttl_ms: u64 },
    /// The store exceeded its capacity limit.
    CapacityExceeded { current: usize, limit: usize },
}

/// Observable state of the Feature Store subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiState)]
pub struct FeatureStoreState {
    /// Current number of entries in the store.
    pub entry_count: usize,
    /// Whether a default TTL is configured.
    pub has_default_ttl: bool,
    /// Default TTL in milliseconds, if set.
    pub default_ttl_ms: Option<u64>,
}
