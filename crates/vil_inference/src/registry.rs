// =============================================================================
// VIL Inference — Model Registry
// =============================================================================
// Central registry for model pools. Supports hot-swap with version tracking.
// =============================================================================

use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;

use crate::backend::{InferError, ModelBackend};
use crate::config::ModelConfig;
use crate::pool::ModelPool;

/// Internal entry tracking a model pool, version, and metadata.
struct ModelEntry {
    pool: Arc<ModelPool>,
    version: u32,
    #[allow(dead_code)]
    loaded_at: Instant,
    #[allow(dead_code)]
    config: ModelConfig,
}

/// Central model registry — register, look up, hot-swap, and remove models.
pub struct ModelRegistry {
    models: DashMap<String, ModelEntry>,
}

impl ModelRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            models: DashMap::new(),
        }
    }

    /// Register a new model by name using a factory function to create pool instances.
    pub fn register(
        &self,
        config: ModelConfig,
        backend_factory: impl Fn() -> Arc<dyn ModelBackend>,
    ) -> Result<(), InferError> {
        if self.models.contains_key(&config.name) {
            return Err(InferError::ExecutionFailed(format!(
                "model '{}' already registered",
                config.name
            )));
        }

        let instances: Vec<Arc<dyn ModelBackend>> =
            (0..config.pool_size).map(|_| backend_factory()).collect();

        let pool = Arc::new(ModelPool::new(&config.name, instances));

        self.models.insert(
            config.name.clone(),
            ModelEntry {
                pool,
                version: 1,
                loaded_at: Instant::now(),
                config,
            },
        );

        Ok(())
    }

    /// Get the model pool for the given name.
    pub fn get(&self, name: &str) -> Option<Arc<ModelPool>> {
        self.models.get(name).map(|e| Arc::clone(&e.pool))
    }

    /// Hot-swap a model's backend instances. Returns the new version number.
    pub fn hot_swap(
        &self,
        name: &str,
        new_backends: Vec<Arc<dyn ModelBackend>>,
    ) -> Result<u32, InferError> {
        let mut entry = self
            .models
            .get_mut(name)
            .ok_or(InferError::ModelNotLoaded)?;

        let new_pool = Arc::new(ModelPool::new(name, new_backends));
        entry.version += 1;
        entry.pool = new_pool;
        entry.loaded_at = Instant::now();

        Ok(entry.version)
    }

    /// List all registered model names.
    pub fn list(&self) -> Vec<String> {
        self.models.iter().map(|e| e.key().clone()).collect()
    }

    /// Get the current version of a model.
    pub fn version(&self, name: &str) -> Option<u32> {
        self.models.get(name).map(|e| e.version)
    }

    /// Remove a model from the registry. Returns true if it existed.
    pub fn remove(&self, name: &str) -> bool {
        self.models.remove(name).is_some()
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}
