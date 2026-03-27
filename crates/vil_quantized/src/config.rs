// =============================================================================
// D13 — Quantized Model Configuration
// =============================================================================

use serde::{Deserialize, Serialize};

use crate::format::QuantFormat;

/// Configuration describing a quantized model's architecture and storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedModelConfig {
    /// Path to the model file (GGUF/GGML)
    pub path: String,
    /// Quantization format of the stored weights
    pub format: QuantFormat,
    /// Maximum context length (tokens)
    pub context_length: usize,
    /// Vocabulary size
    pub vocab_size: usize,
    /// Hidden dimension size
    pub hidden_size: usize,
    /// Number of transformer layers
    pub num_layers: usize,
}

impl QuantizedModelConfig {
    /// Creates a new `QuantizedModelConfigBuilder`.
    pub fn builder() -> QuantizedModelConfigBuilder {
        QuantizedModelConfigBuilder::default()
    }

    /// Estimates the total number of parameters based on architecture.
    ///
    /// Rough estimate: `vocab_size * hidden_size * 2 + num_layers * hidden_size^2 * 4`
    /// (embedding + output projection + 4 weight matrices per layer)
    pub fn estimated_params(&self) -> u64 {
        let embed = (self.vocab_size as u64) * (self.hidden_size as u64) * 2;
        let layers =
            (self.num_layers as u64) * (self.hidden_size as u64) * (self.hidden_size as u64) * 4;
        embed + layers
    }

    /// Estimates memory usage in megabytes.
    pub fn memory_estimate_mb(&self) -> f64 {
        let params = self.estimated_params() as f64;
        let bytes = params * self.format.bytes_per_param();
        bytes / (1024.0 * 1024.0)
    }
}

/// Builder for `QuantizedModelConfig`.
#[derive(Debug, Default)]
pub struct QuantizedModelConfigBuilder {
    path: Option<String>,
    format: Option<QuantFormat>,
    context_length: Option<usize>,
    vocab_size: Option<usize>,
    hidden_size: Option<usize>,
    num_layers: Option<usize>,
}

impl QuantizedModelConfigBuilder {
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn format(mut self, format: QuantFormat) -> Self {
        self.format = Some(format);
        self
    }

    pub fn context_length(mut self, context_length: usize) -> Self {
        self.context_length = Some(context_length);
        self
    }

    pub fn vocab_size(mut self, vocab_size: usize) -> Self {
        self.vocab_size = Some(vocab_size);
        self
    }

    pub fn hidden_size(mut self, hidden_size: usize) -> Self {
        self.hidden_size = Some(hidden_size);
        self
    }

    pub fn num_layers(mut self, num_layers: usize) -> Self {
        self.num_layers = Some(num_layers);
        self
    }

    /// Builds the config. Returns `Err` if any required field is missing.
    pub fn build(self) -> Result<QuantizedModelConfig, String> {
        Ok(QuantizedModelConfig {
            path: self.path.ok_or("path is required")?,
            format: self.format.ok_or("format is required")?,
            context_length: self.context_length.ok_or("context_length is required")?,
            vocab_size: self.vocab_size.ok_or("vocab_size is required")?,
            hidden_size: self.hidden_size.ok_or("hidden_size is required")?,
            num_layers: self.num_layers.ok_or("num_layers is required")?,
        })
    }
}
