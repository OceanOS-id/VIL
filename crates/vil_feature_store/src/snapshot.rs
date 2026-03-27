// =============================================================================
// D16 — Point-in-Time Snapshot
// =============================================================================

use serde::{Deserialize, Serialize};

use crate::store::FeatureValue;

/// A point-in-time snapshot of the feature store contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSnapshot {
    /// Timestamp (ms since epoch) when the snapshot was taken
    pub taken_at: u64,
    /// All features at snapshot time
    pub entries: Vec<(String, FeatureValue)>,
}

impl FeatureSnapshot {
    /// Returns the number of entries in the snapshot.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the snapshot is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
