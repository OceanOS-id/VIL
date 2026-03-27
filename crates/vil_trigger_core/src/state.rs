// =============================================================================
// vil_trigger_core::state — shared trigger state for ServiceProcess metrics
// =============================================================================

use vil_connector_macros::connector_state;

/// Live state metrics for any trigger source.
#[connector_state]
pub struct TriggerState {
    /// Total trigger events fired since startup.
    pub events_fired: u64,
    /// Total trigger errors encountered.
    pub trigger_errors: u64,
    /// Total events dropped due to channel backpressure.
    pub events_dropped: u64,
    /// Timestamp (ns) of the most recent fire (0 if never fired).
    pub last_fire_ns: u64,
}
