// =============================================================================
// vil_trigger_cdc::state — connector state for ServiceProcess health metrics
// =============================================================================

use vil_connector_macros::connector_state;

/// Live state metrics for the CDC trigger.
#[connector_state]
pub struct CdcTriggerState {
    /// Total row change events emitted.
    pub rows_changed: u64,
    /// Total keepalive messages sent to the server.
    pub keepalives_sent: u64,
    /// Total replication stream errors encountered.
    pub stream_errors: u64,
    /// Timestamp (ns) of the most recent row change event (0 if none).
    pub last_event_ns: u64,
}
