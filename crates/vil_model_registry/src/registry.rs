use dashmap::DashMap;

use crate::model::{ModelEntry, ModelStatus};
use crate::version;

/// Versioned model registry. Stores version history per model name.
pub struct ModelRegistry {
    pub models: DashMap<String, Vec<ModelEntry>>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            models: DashMap::new(),
        }
    }

    /// Register a new version of a model. Returns the assigned version number.
    pub fn register(
        &self,
        name: impl Into<String>,
        provider: impl Into<String>,
        config: serde_json::Value,
    ) -> u32 {
        let name = name.into();
        let provider = provider.into();

        let mut entries = self.models.entry(name.clone()).or_default();
        let current_max = entries.iter().map(|e| e.version).max().unwrap_or(0);
        let new_version = version::next_version(current_max);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        entries.push(ModelEntry {
            name,
            version: new_version,
            provider,
            config,
            status: ModelStatus::Staging,
            deployed_at: now,
        });

        new_version
    }

    /// Get the latest active version of a model.
    pub fn get_active(&self, name: &str) -> Option<ModelEntry> {
        self.models.get(name).and_then(|entries| {
            entries
                .iter()
                .filter(|e| e.status == ModelStatus::Active)
                .max_by_key(|e| e.version)
                .cloned()
        })
    }

    /// Promote a specific version to Active status.
    /// Any previously active version for the same model is set to Staging.
    pub fn promote(&self, name: &str, ver: u32) -> Result<(), String> {
        let mut entries = self
            .models
            .get_mut(name)
            .ok_or_else(|| format!("Model '{}' not found", name))?;

        // Demote current active
        for entry in entries.iter_mut() {
            if entry.status == ModelStatus::Active {
                entry.status = ModelStatus::Staging;
            }
        }

        // Promote target
        let target = entries
            .iter_mut()
            .find(|e| e.version == ver)
            .ok_or_else(|| format!("Version {} not found for '{}'", ver, name))?;
        target.status = ModelStatus::Active;

        Ok(())
    }

    /// Rollback to the previous active version.
    /// Finds the highest-versioned Staging entry and promotes it.
    pub fn rollback(&self, name: &str) -> Result<(), String> {
        let mut entries = self
            .models
            .get_mut(name)
            .ok_or_else(|| format!("Model '{}' not found", name))?;

        // Demote current active
        let mut current_active_version = None;
        for entry in entries.iter_mut() {
            if entry.status == ModelStatus::Active {
                current_active_version = Some(entry.version);
                entry.status = ModelStatus::Staging;
            }
        }

        // Find the highest staging version that is NOT the one we just demoted
        let prev = entries
            .iter_mut()
            .filter(|e| {
                e.status == ModelStatus::Staging && Some(e.version) != current_active_version
            })
            .max_by_key(|e| e.version);

        match prev {
            Some(entry) => {
                entry.status = ModelStatus::Active;
                Ok(())
            }
            None => Err(format!("No previous version to rollback to for '{}'", name)),
        }
    }

    /// Deprecate a specific version.
    pub fn deprecate(&self, name: &str, ver: u32) -> Result<(), String> {
        let mut entries = self
            .models
            .get_mut(name)
            .ok_or_else(|| format!("Model '{}' not found", name))?;

        let target = entries
            .iter_mut()
            .find(|e| e.version == ver)
            .ok_or_else(|| format!("Version {} not found for '{}'", ver, name))?;

        target.status = ModelStatus::Deprecated;
        Ok(())
    }

    /// List all models with their version histories.
    pub fn list(&self) -> Vec<(String, Vec<ModelEntry>)> {
        self.models
            .iter()
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect()
    }

    /// Get version history for a specific model.
    pub fn history(&self, name: &str) -> Vec<ModelEntry> {
        self.models
            .get(name)
            .map(|entries| entries.clone())
            .unwrap_or_default()
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}
