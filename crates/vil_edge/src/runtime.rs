//! Edge runtime — lightweight runtime for resource-constrained devices.

use crate::config::{EdgeConfig, TargetArch};
use crate::model::{EdgeModel, ModelFormat, Quantization};

/// Lightweight runtime for edge inference.
#[derive(Debug, Clone)]
pub struct EdgeRuntime {
    pub config: EdgeConfig,
    pub models: Vec<EdgeModel>,
}

impl EdgeRuntime {
    pub fn new(config: EdgeConfig) -> Self {
        Self {
            config,
            models: Vec::new(),
        }
    }

    /// Check if a model can be loaded within the memory budget.
    pub fn can_load(&self, model: &EdgeModel) -> bool {
        if model.size_mb > self.config.max_model_size_mb {
            return false;
        }

        // Account for runtime overhead: model needs ~1.5x its size in memory.
        let estimated_memory = (model.size_mb as f64 * memory_multiplier(model)) as u64;
        let currently_used: u64 = self
            .models
            .iter()
            .map(|m| (m.size_mb as f64 * memory_multiplier(m)) as u64)
            .sum();

        currently_used + estimated_memory <= self.config.max_memory_mb
    }

    /// Rough latency estimate in ms based on arch + model size.
    pub fn estimated_latency_ms(&self, model: &EdgeModel) -> u64 {
        let base_ms_per_mb = match self.config.target_arch {
            TargetArch::X86_64 => 0.5,
            TargetArch::Aarch64 => 0.8,
            TargetArch::Riscv64 => 1.5,
            TargetArch::Wasm32 => 3.0,
        };

        let quant_factor = match model.quantization {
            Quantization::None => 1.0,
            Quantization::Float16 => 0.7,
            Quantization::Int8 => 0.4,
            Quantization::Int4 => 0.25,
        };

        let format_overhead = match model.format {
            ModelFormat::ONNX => 1.0,
            ModelFormat::GGUF => 0.9,
            ModelFormat::SafeTensors => 1.1,
        };

        (model.size_mb as f64 * base_ms_per_mb * quant_factor * format_overhead).ceil() as u64
    }

    /// Register a model if it fits.
    pub fn register_model(&mut self, model: EdgeModel) -> Result<(), EdgeError> {
        if !self.can_load(&model) {
            return Err(EdgeError::InsufficientMemory {
                model_name: model.name.clone(),
                model_size_mb: model.size_mb,
                available_mb: self.config.max_memory_mb,
            });
        }
        self.models.push(model);
        Ok(())
    }
}

/// Memory multiplier based on model quantization.
fn memory_multiplier(model: &EdgeModel) -> f64 {
    match model.quantization {
        Quantization::None => 1.5,
        Quantization::Float16 => 1.3,
        Quantization::Int8 => 1.2,
        Quantization::Int4 => 1.1,
    }
}

/// Edge runtime errors.
#[derive(Debug, Clone)]
pub enum EdgeError {
    InsufficientMemory {
        model_name: String,
        model_size_mb: u64,
        available_mb: u64,
    },
}

impl std::fmt::Display for EdgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsufficientMemory {
                model_name,
                model_size_mb,
                available_mb,
            } => {
                write!(f, "insufficient memory for model '{model_name}': needs {model_size_mb}MB, available {available_mb}MB")
            }
        }
    }
}

impl std::error::Error for EdgeError {}
