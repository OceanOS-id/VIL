// =============================================================================
// vil_trigger_cron::events — connector events emitted on Data Lane
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted each time the cron schedule fires.
#[connector_event]
pub struct CronFired {
    /// FxHash of the cron schedule expression string.
    pub schedule_hash: u32,
    /// Monotonic sequence number for this trigger instance.
    pub sequence: u64,
    /// Scheduled fire time in nanoseconds (UNIX_EPOCH).
    pub scheduled_ns: u64,
    /// Actual fire time in nanoseconds (UNIX_EPOCH).
    pub actual_ns: u64,
}
