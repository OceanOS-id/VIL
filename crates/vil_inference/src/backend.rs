// =============================================================================
// VIL Inference — Model Backend Trait
// =============================================================================
// Abstract over ONNX, candle, or any other inference runtime.
// The actual runtime integration is feature-gated for later.
// =============================================================================

use async_trait::async_trait;
use serde::Serialize;
use std::fmt;
use vil_macros::{VilAiEvent, VilAiFault};

/// Input tensor for inference.
#[derive(Debug, Clone)]
pub struct InferInput {
    pub data: Vec<f32>,
    pub shape: Vec<usize>,
}

/// Output tensor from inference.
#[derive(Debug, Clone, Serialize, VilAiEvent)]
pub struct InferOutput {
    pub data: Vec<f32>,
    pub shape: Vec<usize>,
}

/// Errors that can occur during inference.
#[derive(Debug, Clone, Serialize, VilAiFault)]
pub enum InferError {
    ModelNotLoaded,
    ShapeMismatch {
        expected: Vec<usize>,
        got: Vec<usize>,
    },
    ExecutionFailed(String),
    Timeout,
}

impl fmt::Display for InferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InferError::ModelNotLoaded => write!(f, "model not loaded"),
            InferError::ShapeMismatch { expected, got } => {
                write!(f, "shape mismatch: expected {:?}, got {:?}", expected, got)
            }
            InferError::ExecutionFailed(msg) => write!(f, "execution failed: {}", msg),
            InferError::Timeout => write!(f, "inference timeout"),
        }
    }
}

impl std::error::Error for InferError {}

/// Trait abstracting over any model inference backend (ONNX, candle, etc.).
#[async_trait]
pub trait ModelBackend: Send + Sync {
    /// Run inference on a single input. Returns output tensor as Vec<f32>.
    async fn infer(&self, input: &InferInput) -> Result<InferOutput, InferError>;

    /// Run batched inference. Default: sequential (override for real batching).
    async fn infer_batch(&self, inputs: &[InferInput]) -> Result<Vec<InferOutput>, InferError> {
        let mut results = Vec::with_capacity(inputs.len());
        for input in inputs {
            results.push(self.infer(input).await?);
        }
        Ok(results)
    }

    /// Name of this model.
    fn model_name(&self) -> &str;

    /// Expected input shape, e.g., [1, 768] for embedding.
    fn input_shape(&self) -> &[usize];

    /// Expected output shape, e.g., [1, 384] for embedding.
    fn output_shape(&self) -> &[usize];
}
