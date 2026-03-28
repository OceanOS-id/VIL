// =============================================================================
// vil_trigger_cron::error — CronFault
// =============================================================================
//
// Plain enum, primitives only (no String/Vec).
// Mirrors the #[vil_fault] style prescribed in COMPLIANCE.md §4.
// =============================================================================

use vil_connector_macros::connector_fault;

/// Fault codes specific to the cron trigger.
///
/// All variants carry only primitive types — no heap allocation.
#[connector_fault]
pub enum CronFault {
    /// The supplied cron schedule expression could not be parsed.
    /// `expr_hash` is the FxHash-32 of the offending expression string.
    InvalidSchedule { expr_hash: u32 },

    /// The trigger task was cancelled before it could fire.
    TaskCancelled { trigger_id: u64 },

    /// The event channel is closed — downstream consumer disconnected.
    ChannelClosed { trigger_id: u64 },
}

impl From<CronFault> for vil_trigger_core::TriggerFault {
    fn from(f: CronFault) -> Self {
        match f {
            CronFault::InvalidSchedule { expr_hash } => {
                vil_trigger_core::TriggerFault::ConfigInvalid {
                    field_hash: expr_hash,
                }
            }
            CronFault::TaskCancelled { .. } | CronFault::ChannelClosed { .. } => {
                vil_trigger_core::TriggerFault::IoError {
                    kind_hash: vil_log::dict::register_str("cron"),
                    os_code: 0,
                }
            }
        }
    }
}
