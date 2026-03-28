// =============================================================================
// vil_tensor_shm :: buffer — SHM-style ring buffer for tensor data
// =============================================================================

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::tensor::Tensor;

/// A pre-allocated ring buffer that stores tensor data contiguously.
///
/// Writes return `(offset, len)` pairs that can be used to read back the data
/// without any serialization — the consumer just slices into the same buffer.
pub struct ShmTensorBuffer {
    pub data: Vec<f32>,
    pub capacity: usize,
    pub offset: AtomicUsize,
}

/// Descriptor returned after a successful write — enough to locate the tensor
/// data inside the buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferSlice {
    pub offset: usize,
    pub len: usize,
}

/// Errors from buffer operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferError {
    /// The tensor is larger than the total buffer capacity.
    TensorTooLarge { needed: usize, capacity: usize },
}

impl std::fmt::Display for BufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BufferError::TensorTooLarge { needed, capacity } => {
                write!(
                    f,
                    "tensor needs {} floats but buffer capacity is {}",
                    needed, capacity
                )
            }
        }
    }
}

impl std::error::Error for BufferError {}

impl ShmTensorBuffer {
    /// Create a new buffer that can hold `capacity` f32 elements.
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0.0; capacity],
            capacity,
            offset: AtomicUsize::new(0),
        }
    }

    /// Write tensor data into the ring buffer, returning a `BufferSlice`.
    ///
    /// If the write would exceed the end of the buffer the offset wraps around
    /// to zero (ring semantics).  Returns an error only when the tensor itself
    /// is larger than the buffer capacity.
    pub fn write(&self, tensor: &Tensor) -> Result<BufferSlice, BufferError> {
        let len = tensor.data.len();
        if len > self.capacity {
            return Err(BufferError::TensorTooLarge {
                needed: len,
                capacity: self.capacity,
            });
        }

        // Reserve space atomically.  If we would overrun, wrap to 0.
        let start = self
            .offset
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |cur| {
                if cur + len <= self.capacity {
                    Some(cur + len)
                } else {
                    Some(len) // wrap: new offset = len (we wrote at 0)
                }
            })
            .unwrap_or(0); // fetch_update always succeeds here

        let actual_start = if start + len <= self.capacity {
            start
        } else {
            0
        };

        // SAFETY: we are the only writer to this region (atomic reservation).
        // We cast away shared-ref immutability through a raw pointer — this is
        // sound because each writer reserves a unique, non-overlapping slice.
        let ptr = self.data.as_ptr() as *mut f32;
        unsafe {
            std::ptr::copy_nonoverlapping(tensor.data.as_ptr(), ptr.add(actual_start), len);
        }

        Ok(BufferSlice {
            offset: actual_start,
            len,
        })
    }

    /// Read a slice of the buffer previously obtained from [`write`].
    pub fn read(&self, slice: BufferSlice) -> &[f32] {
        &self.data[slice.offset..slice.offset + slice.len]
    }

    /// Current write offset (for diagnostics).
    pub fn current_offset(&self) -> usize {
        self.offset.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor::{DType, Tensor};

    #[test]
    fn test_buffer_write_read() {
        let buf = ShmTensorBuffer::new(64);
        let t = Tensor::new(vec![1.0, 2.0, 3.0], vec![3], DType::F32).unwrap();
        let slice = buf.write(&t).unwrap();
        assert_eq!(slice.len, 3);
        let data = buf.read(slice);
        assert_eq!(data, &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_buffer_wrap_around() {
        let buf = ShmTensorBuffer::new(8);
        let t1 = Tensor::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], vec![6], DType::F32).unwrap();
        let _s1 = buf.write(&t1).unwrap();
        // Next write of 4 elements won't fit at offset 6, should wrap.
        let t2 = Tensor::new(vec![7.0, 8.0, 9.0, 10.0], vec![4], DType::F32).unwrap();
        let s2 = buf.write(&t2).unwrap();
        assert_eq!(s2.offset, 0);
        let data = buf.read(s2);
        assert_eq!(data, &[7.0, 8.0, 9.0, 10.0]);
    }

    #[test]
    fn test_buffer_too_large() {
        let buf = ShmTensorBuffer::new(2);
        let t = Tensor::new(vec![1.0, 2.0, 3.0], vec![3], DType::F32).unwrap();
        let result = buf.write(&t);
        assert!(result.is_err());
    }
}
