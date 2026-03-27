// =============================================================================
// VIL Inference Server
// =============================================================================
// Model pool, dynamic batching, hot-swap infrastructure.
// Backend-agnostic: trait-based ModelBackend allows plugging in ONNX, candle,
// or any other runtime via feature gates.
// =============================================================================

pub mod backend;
pub mod batcher;
pub mod config;
pub mod noop;
pub mod pool;
pub mod registry;

// Re-exports for convenience
pub use backend::{InferError, InferInput, InferOutput, ModelBackend};
pub use batcher::DynamicBatcher;
pub use config::{InferenceConfig, ModelConfig};
pub use noop::NoOpBackend;
pub use pool::ModelPool;
pub use registry::ModelRegistry;

pub mod semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use plugin::InferencePlugin;
pub use semantic::{InferEvent, InferFault, InferState};

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // ---- NoOpBackend tests ----

    #[tokio::test]
    async fn noop_infer_returns_expected_shape() {
        let backend = NoOpBackend::new("test-model");
        let input = InferInput {
            data: vec![1.0; 768],
            shape: vec![1, 768],
        };
        let output = backend.infer(&input).await.unwrap();
        assert_eq!(output.shape, vec![1, 384]);
        assert_eq!(output.data.len(), 384);
        assert!((output.data[0] - 0.5).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn noop_infer_batch_sequential() {
        let backend = NoOpBackend::new("batch-model");
        let inputs = vec![
            InferInput {
                data: vec![1.0; 768],
                shape: vec![1, 768],
            },
            InferInput {
                data: vec![2.0; 768],
                shape: vec![1, 768],
            },
        ];
        let outputs = backend.infer_batch(&inputs).await.unwrap();
        assert_eq!(outputs.len(), 2);
        for out in &outputs {
            assert_eq!(out.shape, vec![1, 384]);
        }
    }

    #[test]
    fn noop_backend_properties() {
        let backend = NoOpBackend::new("my-model");
        assert_eq!(backend.model_name(), "my-model");
        assert_eq!(backend.input_shape(), &[1, 768]);
        assert_eq!(backend.output_shape(), &[1, 384]);
    }

    // ---- ModelPool tests ----

    #[test]
    fn pool_round_robin() {
        let backends: Vec<Arc<dyn ModelBackend>> = (0..3)
            .map(|i| Arc::new(NoOpBackend::new(&format!("model-{}", i))) as Arc<dyn ModelBackend>)
            .collect();

        let pool = ModelPool::new("test-pool", backends);
        assert_eq!(pool.size(), 3);
        assert_eq!(pool.model_name(), "test-pool");

        // Round-robin: should cycle through 0, 1, 2, 0, 1, 2
        let names: Vec<String> = (0..6).map(|_| pool.get().model_name().to_string()).collect();
        assert_eq!(names[0], "model-0");
        assert_eq!(names[1], "model-1");
        assert_eq!(names[2], "model-2");
        assert_eq!(names[3], "model-0");
        assert_eq!(names[4], "model-1");
        assert_eq!(names[5], "model-2");
    }

    #[test]
    #[should_panic(expected = "at least one instance")]
    fn pool_empty_panics() {
        let _pool = ModelPool::new("empty", vec![]);
    }

    // ---- DynamicBatcher tests ----

    #[tokio::test]
    async fn batcher_single_request() {
        let backend: Arc<dyn ModelBackend> = Arc::new(NoOpBackend::new("batcher-model"));
        let batcher = DynamicBatcher::new(backend, 4, 50);

        let input = InferInput {
            data: vec![1.0; 768],
            shape: vec![1, 768],
        };
        let output = batcher.infer(input).await.unwrap();
        assert_eq!(output.shape, vec![1, 384]);
    }

    #[tokio::test]
    async fn batcher_batch_triggers_at_max_size() {
        let backend: Arc<dyn ModelBackend> = Arc::new(NoOpBackend::new("batch-trigger"));
        let batcher = DynamicBatcher::new(backend, 3, 5000); // long timeout so only size triggers

        let mut handles = vec![];
        for _ in 0..3 {
            let b = Arc::clone(&batcher);
            handles.push(tokio::spawn(async move {
                let input = InferInput {
                    data: vec![1.0; 768],
                    shape: vec![1, 768],
                };
                b.infer(input).await
            }));
        }

        for h in handles {
            let result = h.await.unwrap();
            assert!(result.is_ok());
            assert_eq!(result.unwrap().shape, vec![1, 384]);
        }
    }

    // ---- ModelRegistry tests ----

    #[test]
    fn registry_register_and_get() {
        let registry = ModelRegistry::new();
        let config = ModelConfig {
            name: "embed-v1".into(),
            pool_size: 2,
            max_batch_size: 8,
            max_wait_ms: 10,
            timeout_ms: 5000,
        };

        registry
            .register(config, || Arc::new(NoOpBackend::new("embed-v1")))
            .unwrap();

        let pool = registry.get("embed-v1");
        assert!(pool.is_some());
        assert_eq!(pool.unwrap().size(), 2);

        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn registry_duplicate_register_fails() {
        let registry = ModelRegistry::new();
        let config = ModelConfig {
            name: "dup".into(),
            pool_size: 1,
            ..ModelConfig::default()
        };

        registry
            .register(config.clone(), || Arc::new(NoOpBackend::new("dup")))
            .unwrap();

        let result = registry.register(config, || Arc::new(NoOpBackend::new("dup")));
        assert!(result.is_err());
    }

    #[test]
    fn registry_hot_swap_increments_version() {
        let registry = ModelRegistry::new();
        let config = ModelConfig {
            name: "swap-me".into(),
            pool_size: 1,
            ..ModelConfig::default()
        };

        registry
            .register(config, || Arc::new(NoOpBackend::new("swap-me")))
            .unwrap();

        assert_eq!(registry.version("swap-me"), Some(1));

        let new_backends: Vec<Arc<dyn ModelBackend>> =
            vec![Arc::new(NoOpBackend::new("swap-me-v2"))];
        let v = registry.hot_swap("swap-me", new_backends).unwrap();
        assert_eq!(v, 2);
        assert_eq!(registry.version("swap-me"), Some(2));

        // Hot-swap again
        let newer: Vec<Arc<dyn ModelBackend>> = vec![Arc::new(NoOpBackend::new("swap-me-v3"))];
        let v = registry.hot_swap("swap-me", newer).unwrap();
        assert_eq!(v, 3);
    }

    #[test]
    fn registry_hot_swap_nonexistent_fails() {
        let registry = ModelRegistry::new();
        let result = registry.hot_swap("ghost", vec![Arc::new(NoOpBackend::new("ghost"))]);
        assert!(result.is_err());
    }

    #[test]
    fn registry_list_and_remove() {
        let registry = ModelRegistry::new();

        for name in &["alpha", "beta", "gamma"] {
            let config = ModelConfig {
                name: name.to_string(),
                pool_size: 1,
                ..ModelConfig::default()
            };
            registry
                .register(config, || Arc::new(NoOpBackend::new(name)))
                .unwrap();
        }

        let mut names = registry.list();
        names.sort();
        assert_eq!(names, vec!["alpha", "beta", "gamma"]);

        assert!(registry.remove("beta"));
        assert!(!registry.remove("beta")); // already removed

        let mut names = registry.list();
        names.sort();
        assert_eq!(names, vec!["alpha", "gamma"]);
    }

    // ---- InferError Display tests ----

    #[test]
    fn infer_error_display() {
        assert_eq!(format!("{}", InferError::ModelNotLoaded), "model not loaded");
        assert_eq!(format!("{}", InferError::Timeout), "inference timeout");
        assert_eq!(
            format!("{}", InferError::ExecutionFailed("boom".into())),
            "execution failed: boom"
        );
        assert_eq!(
            format!(
                "{}",
                InferError::ShapeMismatch {
                    expected: vec![1, 768],
                    got: vec![1, 512],
                }
            ),
            "shape mismatch: expected [1, 768], got [1, 512]"
        );
    }

    // ---- Config tests ----

    #[test]
    fn config_default() {
        let c = ModelConfig::default();
        assert_eq!(c.name, "default");
        assert_eq!(c.pool_size, 2);
        assert_eq!(c.max_batch_size, 8);
    }

    #[test]
    fn config_serde_roundtrip() {
        let c = ModelConfig {
            name: "test".into(),
            pool_size: 4,
            max_batch_size: 16,
            max_wait_ms: 20,
            timeout_ms: 3000,
        };
        let json = serde_json::to_string(&c).unwrap();
        let c2: ModelConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(c2.name, "test");
        assert_eq!(c2.pool_size, 4);
    }
}
