//! Model format support for edge inference.

use serde::{Deserialize, Serialize};

/// Supported model formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelFormat {
    ONNX,
    GGUF,
    SafeTensors,
}

impl std::fmt::Display for ModelFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ONNX => write!(f, "ONNX"),
            Self::GGUF => write!(f, "GGUF"),
            Self::SafeTensors => write!(f, "SafeTensors"),
        }
    }
}

/// Quantization level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quantization {
    None,
    Int8,
    Int4,
    Float16,
}

/// An edge-deployable model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeModel {
    pub name: String,
    pub format: ModelFormat,
    pub size_mb: u64,
    pub quantization: Quantization,
}
