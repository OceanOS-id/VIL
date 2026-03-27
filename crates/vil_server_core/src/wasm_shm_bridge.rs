// =============================================================================
// VIL Server — WASM-SHM Memory Bridge
// =============================================================================
//
// Bridges WASM linear memory with VIL ExchangeHeap SHM regions.
//
// Problem: WASM has isolated linear memory. SHM lives in host process memory.
// They can't share pointers directly.
//
// Solution: Copy-on-boundary pattern:
//   Host → WASM: copy from SHM region into WASM linear memory
//   WASM → Host: copy from WASM linear memory into SHM region
//
// The copies happen only at the WASM boundary. Within the host process,
// SHM data is still zero-copy. This is similar to how kernel/userspace
// boundaries work — one copy at the boundary, zero copies within.

use std::sync::Arc;
use vil_shm::{ExchangeHeap, Offset};
use vil_types::RegionId;

/// WASM-SHM bridge for a specific handler.
///
/// Each WASM handler gets its own SHM region for input/output staging.
/// Input:  host writes request body to SHM → copies to WASM memory
/// Output: WASM writes response to its memory → host copies to SHM
pub struct WasmShmBridge {
    heap: Arc<ExchangeHeap>,
    /// Dedicated SHM region for this handler's I/O
    io_region: RegionId,
    /// Handler name (for logging)
    handler_name: String,
    /// Maximum buffer size
    max_buffer_size: usize,
}

impl WasmShmBridge {
    /// Create a new bridge with a dedicated I/O region.
    pub fn new(
        heap: Arc<ExchangeHeap>,
        handler_name: &str,
        region_size: usize,
    ) -> Self {
        let region_name = format!("vil_wasm_io_{}", handler_name);
        let io_region = heap.create_region(&region_name, region_size);

        Self {
            heap,
            io_region,
            handler_name: handler_name.to_string(),
            max_buffer_size: region_size,
        }
    }

    /// Stage request data into SHM for WASM consumption.
    ///
    /// Returns the offset and length where data was written.
    /// The WASM runtime reads from this offset.
    pub fn stage_input(&self, data: &[u8]) -> Result<(Offset, usize), ShmBridgeError> {
        if data.len() > self.max_buffer_size {
            return Err(ShmBridgeError::InputTooLarge {
                size: data.len(),
                max: self.max_buffer_size,
            });
        }

        let offset = self.heap
            .alloc_bytes(self.io_region, data.len(), 8)
            .ok_or(ShmBridgeError::AllocationFailed)?;

        if !self.heap.write_bytes(self.io_region, offset, data) {
            return Err(ShmBridgeError::WriteFailed);
        }

        Ok((offset, data.len()))
    }

    /// Read output data from SHM after WASM execution.
    ///
    /// The WASM runtime writes its response to the specified offset.
    pub fn read_output(&self, offset: Offset, len: usize) -> Result<Vec<u8>, ShmBridgeError> {
        self.heap
            .read_bytes(self.io_region, offset, len)
            .ok_or(ShmBridgeError::ReadFailed)
    }

    /// Reset the I/O region (reclaim all allocations).
    /// Called between requests to prevent region exhaustion.
    pub fn reset(&self) {
        self.heap.reset_region(self.io_region);
    }

    /// Get region statistics.
    pub fn stats(&self) -> Option<vil_shm::RegionStats> {
        self.heap.region_stats(self.io_region)
    }

    pub fn handler_name(&self) -> &str {
        &self.handler_name
    }

    pub fn region_id(&self) -> RegionId {
        self.io_region
    }
}

/// Errors from the WASM-SHM bridge.
#[derive(Debug)]
pub enum ShmBridgeError {
    InputTooLarge { size: usize, max: usize },
    AllocationFailed,
    WriteFailed,
    ReadFailed,
}

impl std::fmt::Display for ShmBridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputTooLarge { size, max } => {
                write!(f, "Input too large: {} bytes (max: {})", size, max)
            }
            Self::AllocationFailed => write!(f, "SHM allocation failed"),
            Self::WriteFailed => write!(f, "SHM write failed"),
            Self::ReadFailed => write!(f, "SHM read failed"),
        }
    }
}

impl std::error::Error for ShmBridgeError {}
