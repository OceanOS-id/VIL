// =============================================================================
// D13 — Quantization Config & Simulated Quantization
// =============================================================================

use serde::{Deserialize, Serialize};
use vil_log::app_log;

use crate::format::QuantFormat;

/// Configuration for a quantization operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizeConfig {
    /// Source model path (e.g., F32 safetensors or GGUF)
    pub source_path: String,
    /// Output path for the quantized model
    pub output_path: String,
    /// Target quantization format
    pub target_format: QuantFormat,
    /// Number of threads to use for quantization
    pub num_threads: usize,
}

/// Result of a (simulated) quantization operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizeResult {
    /// Output file path
    pub output_path: String,
    /// Target format used
    pub format: QuantFormat,
    /// Estimated output size in bytes
    pub estimated_size_bytes: u64,
    /// Whether the operation was simulated (no real model backend)
    pub simulated: bool,
}

/// Simulates quantizing a model from F32 to the target format.
///
/// This is a placeholder — real quantization requires a backend like `candle`.
/// The function estimates the output size based on the parameter count and
/// target format's bytes-per-parameter.
pub fn simulate_quantize(config: &QuantizeConfig, param_count: u64) -> QuantizeResult {
    let size = (param_count as f64 * config.target_format.bytes_per_param()) as u64;

    app_log!(Info, "quantize_simulate", {
        source: config.source_path.clone(),
        output: config.output_path.clone(),
        format: config.target_format.to_string(),
        params: param_count,
        estimated_bytes: size
    });

    QuantizeResult {
        output_path: config.output_path.clone(),
        format: config.target_format,
        estimated_size_bytes: size,
        simulated: true,
    }
}
