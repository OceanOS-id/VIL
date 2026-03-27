// =============================================================================
// D16 — Feature Store (SHM-like ring buffer for feature vectors)
// =============================================================================

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::key::FeatureKey;
use crate::snapshot::FeatureSnapshot;
use crate::ttl;

/// A feature value stored in the feature store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureValue {
    /// The feature data as a vector of f32 values
    pub data: Vec<f32>,
    /// Monotonic version counter
    pub version: u64,
    /// Timestamp in milliseconds since Unix epoch when this value was created
    pub created_at: u64,
    /// Optional TTL in milliseconds. `None` means the entry never expires.
    pub ttl_ms: Option<u64>,
}

/// A concurrent, TTL-aware feature store backed by `DashMap`.
///
/// Features are keyed by compound strings of the form `"entity_id:feature_name"`.
pub struct FeatureStore {
    /// The underlying concurrent map
    pub features: DashMap<String, FeatureValue>,
    /// Default TTL applied to entries that don't specify their own
    pub default_ttl_ms: Option<u64>,
}

impl FeatureStore {
    /// Creates a new empty feature store with no default TTL.
    pub fn new() -> Self {
        Self {
            features: DashMap::new(),
            default_ttl_ms: None,
        }
    }

    /// Creates a new feature store with a default TTL in milliseconds.
    pub fn with_default_ttl(ttl_ms: u64) -> Self {
        Self {
            features: DashMap::new(),
            default_ttl_ms: Some(ttl_ms),
        }
    }

    /// Sets a feature value for the given key.
    ///
    /// If the key already exists, the version is incremented.
    /// The TTL is set from the value's `ttl_ms`, falling back to the store's default.
    pub fn set(&self, key: &FeatureKey, mut value: FeatureValue) {
        let compound = key.to_compound();

        // Resolve TTL: prefer value-level, then store default
        if value.ttl_ms.is_none() {
            value.ttl_ms = self.default_ttl_ms;
        }

        // Set created_at if not already set
        if value.created_at == 0 {
            value.created_at = ttl::now_ms();
        }

        // Increment version if overwriting
        if let Some(existing) = self.features.get(&compound) {
            value.version = existing.version + 1;
        }

        self.features.insert(compound, value);
    }

    /// Gets a cloned feature value for the given key.
    ///
    /// Returns `None` if the key does not exist or the entry has expired.
    pub fn get(&self, key: &FeatureKey) -> Option<FeatureValue> {
        let compound = key.to_compound();
        let entry = self.features.get(&compound)?;
        let val = entry.value();

        if ttl::is_expired(val.created_at, val.ttl_ms) {
            drop(entry);
            self.features.remove(&compound);
            return None;
        }

        Some(val.clone())
    }

    /// Gets multiple feature values at once.
    pub fn get_batch(&self, keys: &[FeatureKey]) -> Vec<Option<FeatureValue>> {
        keys.iter().map(|k| self.get(k)).collect()
    }

    /// Deletes a feature by key. Returns `true` if the key existed.
    pub fn delete(&self, key: &FeatureKey) -> bool {
        let compound = key.to_compound();
        self.features.remove(&compound).is_some()
    }

    /// Returns the number of features currently stored (including possibly expired).
    pub fn count(&self) -> usize {
        self.features.len()
    }

    /// Evicts all expired entries from the store. Returns the number evicted.
    pub fn evict_expired(&self) -> usize {
        let mut evicted = 0;
        let keys_to_remove: Vec<String> = self
            .features
            .iter()
            .filter(|entry| ttl::is_expired(entry.value().created_at, entry.value().ttl_ms))
            .map(|entry| entry.key().clone())
            .collect();

        for key in keys_to_remove {
            self.features.remove(&key);
            evicted += 1;
        }

        evicted
    }

    /// Takes a point-in-time snapshot of all non-expired features.
    pub fn snapshot(&self) -> FeatureSnapshot {
        let now = ttl::now_ms();
        let entries: Vec<(String, FeatureValue)> = self
            .features
            .iter()
            .filter(|entry| !ttl::is_expired(entry.value().created_at, entry.value().ttl_ms))
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        FeatureSnapshot {
            taken_at: now,
            entries,
        }
    }
}

impl Default for FeatureStore {
    fn default() -> Self {
        Self::new()
    }
}
