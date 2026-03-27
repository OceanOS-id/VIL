// =============================================================================
// vil_trigger_fs::state — connector state for ServiceProcess health metrics
// =============================================================================

use vil_connector_macros::connector_state;

/// Live state metrics for the filesystem trigger.
#[connector_state]
pub struct FsTriggerState {
    /// Total filesystem events emitted.
    pub events_emitted: u64,
    /// Total events dropped due to debounce or channel backpressure.
    pub events_dropped: u64,
    /// Total watcher errors encountered.
    pub watcher_errors: u64,
    /// Timestamp (ns) of the most recent event (0 if none).
    pub last_event_ns: u64,
}
