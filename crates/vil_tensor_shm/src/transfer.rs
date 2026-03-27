// =============================================================================
// vil_tensor_shm :: transfer — Zero-copy tensor transfer between services
// =============================================================================

use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::buffer::BufferError;
use crate::pool::{PoolDescriptor, TensorPool};
use crate::tensor::{DType, Tensor};

/// Serialisable descriptor that a producer sends to a consumer so the consumer
/// can locate the tensor inside the shared pool without copying the data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorDescriptor {
    pub buffer_index: usize,
    pub offset: usize,
    pub len: usize,
    pub shape: Vec<usize>,
    pub dtype: DType,
}

impl TensorDescriptor {
    fn from_pool(desc: PoolDescriptor, tensor: &Tensor) -> Self {
        Self {
            buffer_index: desc.buffer_index,
            offset: desc.slice.offset,
            len: desc.slice.len,
            shape: tensor.shape.clone(),
            dtype: tensor.dtype,
        }
    }
}

/// Zero-copy tensor transfer backed by a shared `TensorPool`.
///
/// Writers publish tensors and receive a `TensorDescriptor`; readers use that
/// descriptor to get a direct slice into the pool's memory — no
/// serialization, no allocation.
pub struct TensorTransfer {
    pub pool: Arc<TensorPool>,
    /// Optional log of descriptors for debugging / replay.
    pub log: RwLock<Vec<TensorDescriptor>>,
}

impl TensorTransfer {
    /// Create a transfer channel backed by the given pool.
    pub fn new(pool: Arc<TensorPool>) -> Self {
        Self {
            pool,
            log: RwLock::new(Vec::new()),
        }
    }

    /// Write a tensor into the pool and return a descriptor.
    pub fn send(&self, tensor: &Tensor) -> Result<TensorDescriptor, BufferError> {
        let pool_desc = self.pool.write(tensor)?;
        let td = TensorDescriptor::from_pool(pool_desc, tensor);
        self.log.write().push(td.clone());
        Ok(td)
    }

    /// Read tensor data via a descriptor (zero-copy — returns a slice).
    pub fn receive(&self, desc: &TensorDescriptor) -> &[f32] {
        let pool_desc = PoolDescriptor {
            buffer_index: desc.buffer_index,
            slice: crate::buffer::BufferSlice {
                offset: desc.offset,
                len: desc.len,
            },
        };
        self.pool.read(pool_desc)
    }

    /// Number of descriptors logged so far.
    pub fn transfer_count(&self) -> usize {
        self.log.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor::{DType, Tensor};

    fn make_transfer() -> TensorTransfer {
        let pool = Arc::new(TensorPool::new(2, 256));
        TensorTransfer::new(pool)
    }

    #[test]
    fn test_transfer_send_receive() {
        let tx = make_transfer();
        let t = Tensor::new(vec![1.0, 2.0, 3.0, 4.0], vec![2, 2], DType::F32).unwrap();
        let desc = tx.send(&t).unwrap();
        let data = tx.receive(&desc);
        assert_eq!(data, &[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(desc.shape, vec![2, 2]);
        assert_eq!(desc.dtype, DType::F32);
    }

    #[test]
    fn test_transfer_descriptor_serialization() {
        let tx = make_transfer();
        let t = Tensor::new(vec![5.0, 6.0], vec![2], DType::I32).unwrap();
        let desc = tx.send(&t).unwrap();
        let json = serde_json::to_string(&desc).unwrap();
        let restored: TensorDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.offset, desc.offset);
        assert_eq!(restored.len, desc.len);
        assert_eq!(restored.shape, desc.shape);
        assert_eq!(restored.dtype, DType::I32);
    }

    #[test]
    fn test_transfer_count() {
        let tx = make_transfer();
        let t = Tensor::zeros(vec![4], DType::F32);
        tx.send(&t).unwrap();
        tx.send(&t).unwrap();
        tx.send(&t).unwrap();
        assert_eq!(tx.transfer_count(), 3);
    }

    #[test]
    fn test_concurrent_writes() {
        use std::thread;

        let pool = Arc::new(TensorPool::new(4, 1024));
        let tx = Arc::new(TensorTransfer::new(pool));

        let handles: Vec<_> = (0..8)
            .map(|i| {
                let tx = Arc::clone(&tx);
                thread::spawn(move || {
                    let val = i as f32;
                    let t = Tensor::new(vec![val; 4], vec![4], DType::F32).unwrap();
                    let desc = tx.send(&t).unwrap();
                    let data = tx.receive(&desc);
                    assert_eq!(data.len(), 4);
                    assert!(data.iter().all(|&v| v == val));
                })
            })
            .collect();

        for h in handles {
            h.join().expect("thread panicked");
        }

        assert_eq!(tx.transfer_count(), 8);
    }
}
