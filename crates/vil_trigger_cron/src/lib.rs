// =============================================================================
// vil_trigger_cron — VIL Phase 3 Cron / Schedule Trigger
// =============================================================================
//
// Fires `TriggerEvent` descriptors on cron schedules.  Implements the
// `TriggerSource` trait from `vil_trigger_core`.
//
// # Modules
// - `config`  — CronConfig, MissedFirePolicy
// - `error`   — CronFault (plain enum, no heap)
// - `source`  — CronTrigger: TriggerSource implementation
// - `process` — create_cron_trigger() convenience constructor
//
// # Semantic log
// Every fire emits `mq_log!(Info, MqPayload { ... })` with timing.
// No println!, tracing::info!, log::info! — COMPLIANCE.md §8.
// =============================================================================

pub mod config;
pub mod error;
pub mod process;
pub mod source;

pub use config::{CronConfig, MissedFirePolicy};
pub use error::CronFault;
pub use process::create_cron_trigger;
pub use source::CronTrigger;
