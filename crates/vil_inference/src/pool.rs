// =============================================================================
// VIL Inference — Model Instance Pool
// =============================================================================
// Pre-warmed pool of model backend instances with round-robin dispatch.
// =============================================================================

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::backend::ModelBackend;

/// A pool of pre-warmed model backend instances.
/// Dispatches requests via round-robin for load spreading.
pub struct ModelPool {
    instances: Vec<Arc<dyn ModelBackend>>,
    counter: AtomicUsize,
    model_name: String,
}

impl ModelPool {
    /// Create a new pool with the given model instances.
    ///
    /// # Panics
    /// Panics if `instances` is empty.
    pub fn new(model_name: &str, instances: Vec<Arc<dyn ModelBackend>>) -> Self {
        assert!(
            !instances.is_empty(),
            "ModelPool requires at least one instance"
        );
        Self {
            instances,
            counter: AtomicUsize::new(0),
            model_name: model_name.to_string(),
        }
    }

    /// Get the next backend instance via round-robin.
    pub fn get(&self) -> &Arc<dyn ModelBackend> {
        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % self.instances.len();
        &self.instances[idx]
    }

    /// Number of instances in the pool.
    pub fn size(&self) -> usize {
        self.instances.len()
    }

    /// Name of the model this pool serves.
    pub fn model_name(&self) -> &str {
        &self.model_name
    }
}
