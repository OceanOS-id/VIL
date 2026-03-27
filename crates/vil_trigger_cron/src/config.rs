// =============================================================================
// vil_trigger_cron::config — CronConfig
// =============================================================================
//
// Setup-time configuration (External layout profile — heap types allowed here).
// =============================================================================

/// Policy to apply when a cron fire time is missed (e.g. system was sleeping).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum MissedFirePolicy {
    /// Skip the missed fire(s) and wait for the next scheduled time.
    #[default]
    Skip = 0,

    /// Fire immediately upon discovering the miss, then resume normal schedule.
    FireImmediately = 1,
}

/// Configuration for a `CronTrigger`.
///
/// **Layout profile: External** — used only at initialisation, not on the
/// hot event-emission path.
#[derive(Debug, Clone)]
pub struct CronConfig {
    /// Numeric identity for this trigger instance.
    pub trigger_id: u64,

    /// Standard cron expression (5-field or 6-field with leading seconds).
    ///
    /// Examples:
    /// - `"0 30 6 * * *"` — 06:30 every day (6-field, leading seconds)
    /// - `"*/5 * * * *"`   — every 5 minutes (5-field)
    pub schedule: &'static str,

    /// Policy to apply when a scheduled fire time is missed.
    pub missed_fire: MissedFirePolicy,

    /// Capacity of the internal event channel.
    /// Default 256 is sufficient for most low-frequency cron schedules.
    pub channel_capacity: usize,
}

impl CronConfig {
    /// Construct with sensible defaults.
    pub fn new(trigger_id: u64, schedule: &'static str) -> Self {
        Self {
            trigger_id,
            schedule,
            missed_fire: MissedFirePolicy::Skip,
            channel_capacity: 256,
        }
    }
}
