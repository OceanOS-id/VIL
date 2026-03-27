// =============================================================================
// vil_trigger_cron::process — create_cron_trigger()
// =============================================================================
//
// Convenience constructor: parses the schedule, wires up an mpsc channel, and
// returns a `(CronTrigger, Receiver<TriggerEvent>)` pair ready for use inside
// a VIL ServiceProcess.
//
// Complies with §1 (no raw tokio::spawn for business logic — the task is
// spawned only when `TriggerSource::start()` is called, not here).
// =============================================================================

use tokio::sync::mpsc;

use vil_log::dict::register_str;
use vil_trigger_core::TriggerEvent;

use crate::config::CronConfig;
use crate::error::CronFault;
use crate::source::CronTrigger;

/// Create a `CronTrigger` together with its event receiver channel.
///
/// Returns `CronFault::InvalidSchedule` if the schedule expression cannot be
/// parsed.  Call `TriggerSource::start()` on the returned trigger to begin
/// firing events.
///
/// The `mpsc::Receiver<TriggerEvent>` should be handed to the downstream
/// pipeline stage that will consume events on the Trigger Lane.
pub fn create_cron_trigger(
    config: CronConfig,
) -> Result<(CronTrigger, mpsc::Receiver<TriggerEvent>), CronFault> {
    // Register the schedule expression in the log dict for offline resolution.
    register_str(config.schedule);
    register_str("cron");

    let (tx, rx) = mpsc::channel::<TriggerEvent>(config.channel_capacity);
    let trigger = CronTrigger::new(config, tx)?;
    Ok((trigger, rx))
}
