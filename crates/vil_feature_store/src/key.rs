// =============================================================================
// D16 — Feature Keys
// =============================================================================

use serde::{Deserialize, Serialize};
use std::fmt;

/// A typed key identifying a feature for a specific entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FeatureKey {
    /// The entity identifier (e.g., user ID, session ID)
    pub entity_id: String,
    /// The feature name (e.g., "click_rate_7d", "embedding_v2")
    pub feature_name: String,
}

impl FeatureKey {
    /// Creates a new `FeatureKey`.
    pub fn new(entity_id: impl Into<String>, feature_name: impl Into<String>) -> Self {
        Self {
            entity_id: entity_id.into(),
            feature_name: feature_name.into(),
        }
    }

    /// Returns the compound key string in `"entity_id:feature_name"` format.
    pub fn to_compound(&self) -> String {
        format!("{}:{}", self.entity_id, self.feature_name)
    }

    /// Parses a compound key string back into a `FeatureKey`.
    ///
    /// Returns `None` if the string does not contain exactly one `:`.
    pub fn from_compound(s: &str) -> Option<Self> {
        let mut parts = s.splitn(2, ':');
        let entity_id = parts.next()?.to_string();
        let feature_name = parts.next()?.to_string();
        if entity_id.is_empty() || feature_name.is_empty() {
            return None;
        }
        Some(Self {
            entity_id,
            feature_name,
        })
    }
}

impl fmt::Display for FeatureKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.entity_id, self.feature_name)
    }
}
