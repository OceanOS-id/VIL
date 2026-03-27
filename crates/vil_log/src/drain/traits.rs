// =============================================================================
// vil_log::drain::traits — LogDrain trait
// =============================================================================
//
// Pluggable drain interface. Implementations receive batches of LogSlot
// and are responsible for formatting and writing them out.
// =============================================================================

use async_trait::async_trait;

use crate::types::LogSlot;

/// Pluggable log drain. Receives batches of `LogSlot` from the runtime loop.
///
/// Implementations must be `Send + Sync + 'static` so they can be held
/// in a tokio task.
#[async_trait]
pub trait LogDrain: Send + Sync + 'static {
    /// Human-readable drain name for diagnostics.
    fn name(&self) -> &'static str;

    /// Process a batch of log slots. Called by the runtime drain loop.
    async fn flush(&mut self, batch: &[LogSlot]) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Graceful shutdown. Flush any buffered data, close file handles, etc.
    async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}
