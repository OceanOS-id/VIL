// =============================================================================
// vil_trigger_evm::state — connector state for ServiceProcess health metrics
// =============================================================================

use vil_connector_macros::connector_state;

/// Live state metrics for the EVM trigger.
#[connector_state]
pub struct EvmTriggerState {
    /// Total EVM log events received.
    pub logs_received: u64,
    /// Total decode failures.
    pub decode_errors: u64,
    /// Total subscription reconnections.
    pub reconnections: u64,
    /// Timestamp (ns) of the most recent log event (0 if none).
    pub last_event_ns: u64,
}
