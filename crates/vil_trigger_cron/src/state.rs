// =============================================================================
// vil_trigger_cron::state — connector state for ServiceProcess health metrics
// =============================================================================

use vil_connector_macros::connector_state;

/// Live state metrics for the cron trigger.
#[connector_state]
pub struct CronTriggerState {
    /// Total times the cron schedule has fired.
    pub fires: u64,
    /// Total missed fires (schedule skipped due to backpressure).
    pub missed_fires: u64,
    /// Total errors during fire handling.
    pub fire_errors: u64,
    /// Timestamp (ns) of the most recent fire (0 if never fired).
    pub last_fire_ns: u64,
}
