// =============================================================================
// vil_log::drain::null — NullDrain
// =============================================================================
//
// Discards all log slots. Used for benchmarking baseline ring throughput.
// =============================================================================

use async_trait::async_trait;

use crate::drain::traits::LogDrain;
use crate::types::LogSlot;

/// Drain that discards everything. Zero allocation, zero I/O.
pub struct NullDrain;

#[async_trait]
impl LogDrain for NullDrain {
    fn name(&self) -> &'static str {
        "null"
    }

    async fn flush(&mut self, _batch: &[LogSlot]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}
