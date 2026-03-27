// =============================================================================
// D13 — GGUF/GGML Quantization Format Types
// =============================================================================

use serde::{Deserialize, Serialize};
use std::fmt;

/// Quantization format representing the numerical precision of model weights.
///
/// Each variant corresponds to a specific bit-width and quantization scheme
/// used in GGUF/GGML model files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuantFormat {
    /// Full 32-bit floating point (no quantization)
    F32,
    /// Half-precision 16-bit floating point
    F16,
    /// 8-bit quantization, scheme 0 (block quantization)
    Q8_0,
    /// 4-bit quantization, scheme 0 (absmax block quantization)
    Q4_0,
    /// 4-bit quantization, scheme 1 (absmax + offset)
    Q4_1,
    /// 5-bit quantization, scheme 0
    Q5_0,
    /// 5-bit quantization, scheme 1
    Q5_1,
}

impl QuantFormat {
    /// Returns the effective bytes per parameter for this quantization format.
    ///
    /// These values are approximate and match typical GGML quantization overhead:
    /// - F32: 4.0 bytes/param
    /// - F16: 2.0 bytes/param
    /// - Q8_0: 1.0 byte/param
    /// - Q4_0: 0.5 bytes/param
    /// - Q4_1: 0.5625 bytes/param (4 bits + small overhead)
    /// - Q5_0: 0.625 bytes/param
    /// - Q5_1: 0.6875 bytes/param
    pub fn bytes_per_param(&self) -> f64 {
        match self {
            QuantFormat::F32 => 4.0,
            QuantFormat::F16 => 2.0,
            QuantFormat::Q8_0 => 1.0,
            QuantFormat::Q4_0 => 0.5,
            QuantFormat::Q4_1 => 0.5625,
            QuantFormat::Q5_0 => 0.625,
            QuantFormat::Q5_1 => 0.6875,
        }
    }

    /// Returns a human-readable label for the format.
    pub fn label(&self) -> &'static str {
        match self {
            QuantFormat::F32 => "F32 (32-bit float)",
            QuantFormat::F16 => "F16 (16-bit float)",
            QuantFormat::Q8_0 => "Q8_0 (8-bit quantized)",
            QuantFormat::Q4_0 => "Q4_0 (4-bit quantized)",
            QuantFormat::Q4_1 => "Q4_1 (4-bit quantized + offset)",
            QuantFormat::Q5_0 => "Q5_0 (5-bit quantized)",
            QuantFormat::Q5_1 => "Q5_1 (5-bit quantized + offset)",
        }
    }

    /// Returns all supported quantization formats.
    pub fn all() -> &'static [QuantFormat] {
        &[
            QuantFormat::F32,
            QuantFormat::F16,
            QuantFormat::Q8_0,
            QuantFormat::Q4_0,
            QuantFormat::Q4_1,
            QuantFormat::Q5_0,
            QuantFormat::Q5_1,
        ]
    }
}

impl fmt::Display for QuantFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QuantFormat::F32 => write!(f, "F32"),
            QuantFormat::F16 => write!(f, "F16"),
            QuantFormat::Q8_0 => write!(f, "Q8_0"),
            QuantFormat::Q4_0 => write!(f, "Q4_0"),
            QuantFormat::Q4_1 => write!(f, "Q4_1"),
            QuantFormat::Q5_0 => write!(f, "Q5_0"),
            QuantFormat::Q5_1 => write!(f, "Q5_1"),
        }
    }
}
