// =============================================================================
// vil_tensor_shm — Zero-copy tensor serving via SHM-mapped buffers
// =============================================================================
//
// Eliminates serialization between inference stages by storing tensors in
// pre-allocated ring buffers and passing lightweight descriptors instead of
// copying data.
//
// # Modules
//
// - `tensor`   — Core `Tensor` type with shape, data, and `DType`.
// - `buffer`   — `ShmTensorBuffer` — ring buffer for contiguous tensor storage.
// - `pool`     — `TensorPool` — round-robin pool of buffers.
// - `transfer` — `TensorTransfer` — zero-copy send/receive via descriptors.

pub mod buffer;
pub mod pool;
pub mod tensor;
pub mod transfer;

// Re-exports for convenience.
pub use buffer::{BufferSlice, ShmTensorBuffer};
pub use pool::{PoolDescriptor, TensorPool};
pub use tensor::{DType, Tensor, TensorError};
pub use transfer::{TensorDescriptor, TensorTransfer};

pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod semantic;

pub use plugin::TensorShmPlugin;
pub use semantic::{TensorAllocEvent, TensorFault, TensorPoolState};
