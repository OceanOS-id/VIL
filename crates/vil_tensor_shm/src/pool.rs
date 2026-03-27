// =============================================================================
// vil_tensor_shm :: pool — Round-robin tensor buffer pool
// =============================================================================

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::buffer::{BufferError, BufferSlice, ShmTensorBuffer};
use crate::tensor::Tensor;

/// Descriptor that identifies a tensor's location within the pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolDescriptor {
    pub buffer_index: usize,
    pub slice: BufferSlice,
}

/// A pool of `ShmTensorBuffer`s with round-robin allocation.
pub struct TensorPool {
    pub buffers: Vec<ShmTensorBuffer>,
    pub counter: AtomicUsize,
}

impl TensorPool {
    /// Create a pool with `n` buffers, each holding `capacity` f32 elements.
    pub fn new(n: usize, capacity: usize) -> Self {
        let buffers = (0..n).map(|_| ShmTensorBuffer::new(capacity)).collect();
        Self {
            buffers,
            counter: AtomicUsize::new(0),
        }
    }

    /// Number of buffers in the pool.
    pub fn len(&self) -> usize {
        self.buffers.len()
    }

    /// Returns true if the pool has no buffers.
    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }

    /// Write a tensor into the next buffer (round-robin) and return a
    /// descriptor.
    pub fn write(&self, tensor: &Tensor) -> Result<PoolDescriptor, BufferError> {
        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % self.buffers.len();
        let slice = self.buffers[idx].write(tensor)?;
        Ok(PoolDescriptor {
            buffer_index: idx,
            slice,
        })
    }

    /// Read tensor data using a previously obtained descriptor.
    pub fn read(&self, desc: PoolDescriptor) -> &[f32] {
        self.buffers[desc.buffer_index].read(desc.slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor::{DType, Tensor};

    #[test]
    fn test_pool_round_robin() {
        let pool = TensorPool::new(3, 64);
        let t = Tensor::new(vec![1.0, 2.0], vec![2], DType::F32).unwrap();

        let d0 = pool.write(&t).unwrap();
        let d1 = pool.write(&t).unwrap();
        let d2 = pool.write(&t).unwrap();
        let d3 = pool.write(&t).unwrap();

        assert_eq!(d0.buffer_index, 0);
        assert_eq!(d1.buffer_index, 1);
        assert_eq!(d2.buffer_index, 2);
        assert_eq!(d3.buffer_index, 0); // wraps
    }

    #[test]
    fn test_pool_write_and_read() {
        let pool = TensorPool::new(2, 64);
        let t = Tensor::new(vec![3.0, 4.0, 5.0], vec![3], DType::F32).unwrap();
        let desc = pool.write(&t).unwrap();
        let data = pool.read(desc);
        assert_eq!(data, &[3.0, 4.0, 5.0]);
    }
}
