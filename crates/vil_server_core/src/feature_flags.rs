// =============================================================================
// VIL Server — Feature Flags
// =============================================================================
//
// Runtime feature toggling without redeployment.
// Supports: boolean flags, percentage rollouts, user-based targeting.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Feature flag definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlag {
    pub name: String,
    pub enabled: bool,
    pub description: String,
    /// Percentage rollout (0-100). Only applicable if enabled=true.
    pub rollout_percentage: u8,
    /// Target specific user/tenant IDs (empty = all)
    pub target_ids: Vec<String>,
}

/// Feature flag store.
pub struct FeatureFlags {
    flags: Arc<DashMap<String, FeatureFlag>>,
}

impl FeatureFlags {
    pub fn new() -> Self {
        Self {
            flags: Arc::new(DashMap::new()),
        }
    }

    /// Define a feature flag.
    pub fn define(&self, name: &str, enabled: bool, description: &str) {
        self.flags.insert(
            name.to_string(),
            FeatureFlag {
                name: name.to_string(),
                enabled,
                description: description.to_string(),
                rollout_percentage: 100,
                target_ids: Vec::new(),
            },
        );
    }

    /// Define a flag with percentage rollout.
    pub fn define_rollout(&self, name: &str, percentage: u8, description: &str) {
        self.flags.insert(
            name.to_string(),
            FeatureFlag {
                name: name.to_string(),
                enabled: true,
                description: description.to_string(),
                rollout_percentage: percentage,
                target_ids: Vec::new(),
            },
        );
    }

    /// Check if a flag is enabled (simple boolean check).
    pub fn is_enabled(&self, name: &str) -> bool {
        self.flags.get(name).map(|f| f.enabled).unwrap_or(false)
    }

    /// Check if a flag is enabled for a specific user/entity.
    /// Uses percentage rollout and target list.
    pub fn is_enabled_for(&self, name: &str, entity_id: &str) -> bool {
        match self.flags.get(name) {
            Some(flag) => {
                if !flag.enabled {
                    return false;
                }
                // Check target list
                if !flag.target_ids.is_empty() {
                    return flag.target_ids.contains(&entity_id.to_string());
                }
                // Check percentage rollout
                if flag.rollout_percentage >= 100 {
                    return true;
                }
                // Deterministic hash for consistent rollout
                let hash = simple_hash(entity_id) % 100;
                hash < flag.rollout_percentage as u64
            }
            None => false,
        }
    }

    /// Toggle a flag on/off.
    pub fn toggle(&self, name: &str) -> Option<bool> {
        self.flags.get_mut(name).map(|mut f| {
            f.enabled = !f.enabled;
            f.enabled
        })
    }

    /// Set a flag's enabled state.
    pub fn set_enabled(&self, name: &str, enabled: bool) {
        if let Some(mut f) = self.flags.get_mut(name) {
            f.enabled = enabled;
        }
    }

    /// List all flags.
    pub fn list(&self) -> Vec<FeatureFlag> {
        self.flags.iter().map(|e| e.value().clone()).collect()
    }

    /// Get flag count.
    pub fn count(&self) -> usize {
        self.flags.len()
    }

    /// Load flags from JSON.
    pub fn load_json(&self, json: &str) -> Result<usize, String> {
        let flags: Vec<FeatureFlag> = serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse feature flags: {}", e))?;
        let count = flags.len();
        for flag in flags {
            self.flags.insert(flag.name.clone(), flag);
        }
        Ok(count)
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self::new()
    }
}

fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for b in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    hash
}
