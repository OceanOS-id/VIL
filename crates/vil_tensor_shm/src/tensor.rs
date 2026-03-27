// =============================================================================
// vil_tensor_shm :: tensor — Core tensor type with shape, data, and dtype
// =============================================================================

use serde::{Deserialize, Serialize};

/// Data type for tensor elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DType {
    F32,
    F16,
    I32,
    U8,
}

impl DType {
    /// Returns the size in bytes of a single element of this dtype.
    pub fn element_size(&self) -> usize {
        match self {
            DType::F32 => 4,
            DType::F16 => 2,
            DType::I32 => 4,
            DType::U8 => 1,
        }
    }
}

/// A tensor with shape metadata and f32 data storage.
///
/// Data is stored as `Vec<f32>` regardless of dtype — the dtype field records
/// the *logical* element type for downstream consumers (e.g. quantised models).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tensor {
    pub data: Vec<f32>,
    pub shape: Vec<usize>,
    pub dtype: DType,
}

impl Tensor {
    /// Create a new tensor, validating that data length matches shape.
    ///
    /// Returns `Err` if the product of `shape` dimensions does not equal
    /// `data.len()`.
    pub fn new(data: Vec<f32>, shape: Vec<usize>, dtype: DType) -> Result<Self, TensorError> {
        let expected: usize = shape.iter().product();
        if expected != data.len() {
            return Err(TensorError::ShapeMismatch {
                expected,
                got: data.len(),
            });
        }
        Ok(Self { data, shape, dtype })
    }

    /// Create a zero-filled tensor with the given shape.
    pub fn zeros(shape: Vec<usize>, dtype: DType) -> Self {
        let len: usize = shape.iter().product();
        Self {
            data: vec![0.0; len],
            shape,
            dtype,
        }
    }

    /// Total number of elements.
    pub fn numel(&self) -> usize {
        self.data.len()
    }

    /// Total size in bytes (logical, based on dtype).
    pub fn byte_size(&self) -> usize {
        self.numel() * self.dtype.element_size()
    }

    /// Number of dimensions.
    pub fn ndim(&self) -> usize {
        self.shape.len()
    }

    /// Returns true if the tensor has zero elements.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// Errors arising from tensor operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TensorError {
    ShapeMismatch { expected: usize, got: usize },
}

impl std::fmt::Display for TensorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TensorError::ShapeMismatch { expected, got } => {
                write!(
                    f,
                    "shape expects {} elements but data has {}",
                    expected, got
                )
            }
        }
    }
}

impl std::error::Error for TensorError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_creation() {
        let t = Tensor::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], vec![2, 3], DType::F32)
            .expect("valid tensor");
        assert_eq!(t.numel(), 6);
        assert_eq!(t.ndim(), 2);
        assert_eq!(t.shape, vec![2, 3]);
    }

    #[test]
    fn test_shape_validation_fails() {
        let result = Tensor::new(vec![1.0, 2.0, 3.0], vec![2, 3], DType::F32);
        assert!(result.is_err());
        match result.unwrap_err() {
            TensorError::ShapeMismatch { expected, got } => {
                assert_eq!(expected, 6);
                assert_eq!(got, 3);
            }
        }
    }

    #[test]
    fn test_zeros() {
        let t = Tensor::zeros(vec![3, 4], DType::F32);
        assert_eq!(t.numel(), 12);
        assert!(t.data.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_empty_tensor() {
        let t = Tensor::new(vec![], vec![0], DType::F32).expect("empty tensor");
        assert!(t.is_empty());
        assert_eq!(t.numel(), 0);
    }

    #[test]
    fn test_dtype_element_size() {
        assert_eq!(DType::F32.element_size(), 4);
        assert_eq!(DType::F16.element_size(), 2);
        assert_eq!(DType::I32.element_size(), 4);
        assert_eq!(DType::U8.element_size(), 1);
    }

    #[test]
    fn test_byte_size() {
        let t = Tensor::zeros(vec![10], DType::F16);
        // 10 elements * 2 bytes = 20
        assert_eq!(t.byte_size(), 20);
    }
}
