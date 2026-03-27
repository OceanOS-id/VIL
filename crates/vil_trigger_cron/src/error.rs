// =============================================================================
// vil_trigger_cron::error — CronFault
// =============================================================================
//
// Plain enum, primitives only (no String/Vec).
// Mirrors the #[vil_fault] style prescribed in COMPLIANCE.md §4.
// =============================================================================

/// Fault codes specific to the cron trigger.
///
/// All variants carry only primitive types — no heap allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
                vil_trigger_core::TriggerFault::ConfigInvalid { field_hash: expr_hash }
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

impl core::fmt::Display for CronFault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidSchedule { expr_hash } => {
                write!(f, "CronFault::InvalidSchedule(expr={expr_hash:#x})")
            }
            Self::TaskCancelled { trigger_id } => {
                write!(f, "CronFault::TaskCancelled(id={trigger_id})")
            }
            Self::ChannelClosed { trigger_id } => {
                write!(f, "CronFault::ChannelClosed(id={trigger_id})")
            }
        }
    }
}
