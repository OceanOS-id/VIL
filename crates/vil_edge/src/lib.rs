//! # vil_edge
//!
//! N09 — Edge Inference: lightweight runtime for resource-constrained edge devices.
//! Supports ONNX, GGUF, SafeTensors formats with memory budget enforcement.

pub mod config;
pub mod model;
pub mod runtime;

pub use config::{EdgeConfig, TargetArch};
pub use model::{EdgeModel, ModelFormat, Quantization};
pub use runtime::{EdgeError, EdgeRuntime};

// VIL integration layer
pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod vil_semantic;

pub use plugin::EdgePlugin;
pub use vil_semantic::{EdgeEvent, EdgeFault, EdgeState};

#[cfg(test)]
mod tests {
    use super::*;

    fn small_model() -> EdgeModel {
        EdgeModel {
            name: "tiny-llm".into(),
            format: ModelFormat::GGUF,
            size_mb: 50,
            quantization: Quantization::Int4,
        }
    }

    fn large_model() -> EdgeModel {
        EdgeModel {
            name: "big-llm".into(),
            format: ModelFormat::ONNX,
            size_mb: 2000,
            quantization: Quantization::None,
        }
    }

    #[test]
    fn test_memory_budget_accepts_small_model() {
        let rt = EdgeRuntime::new(EdgeConfig {
            max_memory_mb: 512,
            max_model_size_mb: 256,
            target_arch: TargetArch::Aarch64,
        });
        assert!(rt.can_load(&small_model()));
    }

    #[test]
    fn test_memory_budget_rejects_large_model() {
        let rt = EdgeRuntime::new(EdgeConfig {
            max_memory_mb: 512,
            max_model_size_mb: 256,
            target_arch: TargetArch::Aarch64,
        });
        assert!(!rt.can_load(&large_model()));
    }

    #[test]
    fn test_arch_latency_varies() {
        let model = small_model();
        let fast = EdgeRuntime::new(EdgeConfig {
            target_arch: TargetArch::X86_64,
            ..EdgeConfig::default()
        });
        let slow = EdgeRuntime::new(EdgeConfig {
            target_arch: TargetArch::Wasm32,
            ..EdgeConfig::default()
        });
        assert!(fast.estimated_latency_ms(&model) < slow.estimated_latency_ms(&model));
    }

    #[test]
    fn test_model_format_display() {
        assert_eq!(format!("{}", ModelFormat::ONNX), "ONNX");
        assert_eq!(format!("{}", ModelFormat::GGUF), "GGUF");
        assert_eq!(format!("{}", ModelFormat::SafeTensors), "SafeTensors");
    }

    #[test]
    fn test_register_model_success() {
        let mut rt = EdgeRuntime::new(EdgeConfig::default());
        assert!(rt.register_model(small_model()).is_ok());
        assert_eq!(rt.models.len(), 1);
    }

    #[test]
    fn test_register_model_too_large() {
        let mut rt = EdgeRuntime::new(EdgeConfig {
            max_memory_mb: 100,
            max_model_size_mb: 50,
            target_arch: TargetArch::Aarch64,
        });
        assert!(rt.register_model(large_model()).is_err());
    }

    #[test]
    fn test_quantization_affects_latency() {
        let rt = EdgeRuntime::new(EdgeConfig::default());
        let q_none = EdgeModel {
            name: "m".into(),
            format: ModelFormat::ONNX,
            size_mb: 100,
            quantization: Quantization::None,
        };
        let q_int4 = EdgeModel {
            name: "m".into(),
            format: ModelFormat::ONNX,
            size_mb: 100,
            quantization: Quantization::Int4,
        };
        assert!(rt.estimated_latency_ms(&q_int4) < rt.estimated_latency_ms(&q_none));
    }

    #[test]
    fn test_target_arch_display() {
        assert_eq!(format!("{}", TargetArch::X86_64), "x86_64");
        assert_eq!(format!("{}", TargetArch::Riscv64), "riscv64");
    }

    #[test]
    fn test_cumulative_memory_budget() {
        let mut rt = EdgeRuntime::new(EdgeConfig {
            max_memory_mb: 100,
            max_model_size_mb: 60,
            target_arch: TargetArch::Aarch64,
        });
        let m1 = EdgeModel {
            name: "a".into(),
            format: ModelFormat::GGUF,
            size_mb: 50,
            quantization: Quantization::Int4,
        };
        let m2 = EdgeModel {
            name: "b".into(),
            format: ModelFormat::GGUF,
            size_mb: 50,
            quantization: Quantization::Int4,
        };
        assert!(rt.register_model(m1).is_ok());
        // Second model should fail due to cumulative memory.
        assert!(!rt.can_load(&m2));
    }
}
