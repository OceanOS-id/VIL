// =============================================================================
// crates/vil_types/src/faults.rs
// =============================================================================

use serde::{Deserialize, Serialize};

/// High Availability (HA) Failover Strategy Intent.
/// Determines how a workflow will recover when a Fault occurs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FailoverStrategy {
    /// Immediately route to a backup instance.
    ImmediateBackup,

    /// Retry payload delivery several times before failing.
    Retry { max_attempts: u32, backoff_ms: u64 },

    /// Custom user-defined strategy.
    Custom(String),
}

/// Trait that must be implemented by all `#[vil_fault]` messages.
/// Governs the error lifecycle within the VIL Control Lane.
pub trait FaultHandler {
    /// Emit an error alert without disrupting the data pipeline.
    fn signal_error(&self);

    /// Halt execution of the workflow/instance associated with this session.
    fn control_abort(&self, session_id: u64);

    /// Reduce service quality level (Graceful Degradation).
    fn degrade(&self, level: u8);
}
