// =============================================================================
// VIL Inference — Mock Backend for Testing
// =============================================================================

use async_trait::async_trait;
use std::time::Duration;

use crate::backend::{InferError, InferInput, InferOutput, ModelBackend};

/// A no-op model backend that provides inference without a real model with configurable latency.
pub struct NoOpBackend {
    name: String,
    input_shape: Vec<usize>,
    output_shape: Vec<usize>,
    latency_ms: u64,
}

impl NoOpBackend {
    /// Create a no-op backend with default shapes (input: [1,768], output: [1,384]).
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            input_shape: vec![1, 768],
            output_shape: vec![1, 384],
            latency_ms: 1,
        }
    }

    /// Set artificial latency in milliseconds.
    pub fn latency(mut self, ms: u64) -> Self {
        self.latency_ms = ms;
        self
    }

    /// Set custom input shape.
    pub fn with_input_shape(mut self, shape: Vec<usize>) -> Self {
        self.input_shape = shape;
        self
    }

    /// Set custom output shape.
    pub fn with_output_shape(mut self, shape: Vec<usize>) -> Self {
        self.output_shape = shape;
        self
    }
}

#[async_trait]
impl ModelBackend for NoOpBackend {
    async fn infer(&self, _input: &InferInput) -> Result<InferOutput, InferError> {
        tokio::time::sleep(Duration::from_millis(self.latency_ms)).await;
        let total_elements: usize = self.output_shape.iter().product();
        Ok(InferOutput {
            data: vec![0.5; total_elements],
            shape: self.output_shape.clone(),
        })
    }

    fn model_name(&self) -> &str {
        &self.name
    }

    fn input_shape(&self) -> &[usize] {
        &self.input_shape
    }

    fn output_shape(&self) -> &[usize] {
        &self.output_shape
    }
}
