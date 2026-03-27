// =============================================================================
// vil_quantized — D13: Model Quantization Runtime
// =============================================================================
//! Infrastructure for quantized model loading, configuration, and inference.
//!
//! This crate provides the types and runtime scaffolding for working with
//! GGUF/GGML quantized models. Actual inference is simulated — a production
//! deployment would integrate `candle` or a similar backend.

pub mod config;
pub mod format;
pub mod quantize;
pub mod runtime;
pub mod semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

// Re-exports for convenience
pub use config::{QuantizedModelConfig, QuantizedModelConfigBuilder};
pub use format::QuantFormat;
pub use quantize::{simulate_quantize, QuantizeConfig, QuantizeResult};
pub use runtime::QuantizedRuntime;
pub use plugin::QuantizedPlugin;
pub use semantic::{QuantizeEvent, QuantizeFault, QuantizeFaultType, QuantizedState};

#[cfg(test)]
mod tests {
    use super::*;

    // ── Format tests ──────────────────────────────────────────────────

    #[test]
    fn test_format_bytes_per_param_f32() {
        assert!((QuantFormat::F32.bytes_per_param() - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_format_bytes_per_param_f16() {
        assert!((QuantFormat::F16.bytes_per_param() - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_format_bytes_per_param_q8_0() {
        assert!((QuantFormat::Q8_0.bytes_per_param() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_format_bytes_per_param_q4_0() {
        assert!((QuantFormat::Q4_0.bytes_per_param() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_format_display() {
        assert_eq!(format!("{}", QuantFormat::Q4_0), "Q4_0");
        assert_eq!(format!("{}", QuantFormat::F16), "F16");
        assert_eq!(format!("{}", QuantFormat::Q5_1), "Q5_1");
    }

    #[test]
    fn test_format_all_variants() {
        let all = QuantFormat::all();
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn test_format_serde_roundtrip() {
        let fmt = QuantFormat::Q4_1;
        let json = serde_json::to_string(&fmt).unwrap();
        let back: QuantFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(fmt, back);
    }

    // ── Memory estimation tests ───────────────────────────────────────

    /// A ~7B parameter model at Q4_0 should be roughly 3.5 GB.
    #[test]
    fn test_memory_estimate_7b_q4_0() {
        let config = QuantizedModelConfig::builder()
            .path("model.gguf")
            .format(QuantFormat::Q4_0)
            .context_length(4096)
            .vocab_size(32000)
            .hidden_size(4096)
            .num_layers(32)
            .build()
            .unwrap();

        let mb = config.memory_estimate_mb();
        let gb = mb / 1024.0;
        // 7B Q4_0: expect ~3.0-4.0 GB range
        assert!(gb > 1.0, "7B Q4_0 should be > 1 GB, got {:.2} GB", gb);
        assert!(gb < 6.0, "7B Q4_0 should be < 6 GB, got {:.2} GB", gb);
    }

    /// A ~7B parameter model at Q8_0 should be roughly 7 GB.
    #[test]
    fn test_memory_estimate_7b_q8_0() {
        let config = QuantizedModelConfig::builder()
            .path("model.gguf")
            .format(QuantFormat::Q8_0)
            .context_length(4096)
            .vocab_size(32000)
            .hidden_size(4096)
            .num_layers(32)
            .build()
            .unwrap();

        let mb = config.memory_estimate_mb();
        let gb = mb / 1024.0;
        // Q8_0 should be ~2x Q4_0
        assert!(gb > 2.0, "7B Q8_0 should be > 2 GB, got {:.2} GB", gb);
        assert!(gb < 12.0, "7B Q8_0 should be < 12 GB, got {:.2} GB", gb);
    }

    // ── Config builder tests ──────────────────────────────────────────

    #[test]
    fn test_config_builder_success() {
        let config = QuantizedModelConfig::builder()
            .path("/models/llama-7b.gguf")
            .format(QuantFormat::Q4_0)
            .context_length(4096)
            .vocab_size(32000)
            .hidden_size(4096)
            .num_layers(32)
            .build();

        assert!(config.is_ok());
        let c = config.unwrap();
        assert_eq!(c.path, "/models/llama-7b.gguf");
        assert_eq!(c.format, QuantFormat::Q4_0);
        assert_eq!(c.num_layers, 32);
    }

    #[test]
    fn test_config_builder_missing_field() {
        let result = QuantizedModelConfig::builder()
            .path("model.gguf")
            // missing format and other fields
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_serde_roundtrip() {
        let config = QuantizedModelConfig::builder()
            .path("test.gguf")
            .format(QuantFormat::F16)
            .context_length(2048)
            .vocab_size(50000)
            .hidden_size(768)
            .num_layers(12)
            .build()
            .unwrap();

        let json = serde_json::to_string(&config).unwrap();
        let back: QuantizedModelConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.path, "test.gguf");
        assert_eq!(back.format, QuantFormat::F16);
        assert_eq!(back.hidden_size, 768);
    }

    // ── Runtime tests ─────────────────────────────────────────────────

    #[test]
    fn test_runtime_not_loaded_by_default() {
        let config = make_test_config();
        let rt = QuantizedRuntime::new(config);
        assert!(!rt.is_loaded());
    }

    #[test]
    fn test_runtime_generate_fails_when_not_loaded() {
        let config = make_test_config();
        let rt = QuantizedRuntime::new(config);
        let result = rt.generate("hello", 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_runtime_load_and_generate() {
        let config = make_test_config();
        let mut rt = QuantizedRuntime::new(config);
        rt.load().unwrap();
        assert!(rt.is_loaded());

        let output = rt.generate("Tell me a story", 100).unwrap();
        assert!(output.contains("simulated"));
        assert!(output.contains("Q4_0"));
    }

    #[test]
    fn test_runtime_memory_estimate() {
        let config = make_test_config();
        let rt = QuantizedRuntime::new(config);
        let mb = rt.memory_estimate_mb();
        assert!(mb > 0.0);
    }

    // ── Quantize simulation tests ─────────────────────────────────────

    #[test]
    fn test_simulate_quantize() {
        let qc = QuantizeConfig {
            source_path: "model_f32.bin".into(),
            output_path: "model_q4.gguf".into(),
            target_format: QuantFormat::Q4_0,
            num_threads: 4,
        };
        let result = simulate_quantize(&qc, 7_000_000_000);
        assert!(result.simulated);
        assert_eq!(result.format, QuantFormat::Q4_0);
        // 7B * 0.5 bytes = 3.5 GB
        let gb = result.estimated_size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        assert!(
            (gb - 3.2596).abs() < 0.5,
            "Expected ~3.26 GB, got {:.4} GB",
            gb
        );
    }

    // ── Helper ────────────────────────────────────────────────────────

    fn make_test_config() -> QuantizedModelConfig {
        QuantizedModelConfig::builder()
            .path("test-model.gguf")
            .format(QuantFormat::Q4_0)
            .context_length(4096)
            .vocab_size(32000)
            .hidden_size(4096)
            .num_layers(32)
            .build()
            .unwrap()
    }
}
