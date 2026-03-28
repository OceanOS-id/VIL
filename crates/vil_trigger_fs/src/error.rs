// =============================================================================
// vil_trigger_fs::error — FsFault
// =============================================================================
//
// Plain enum, primitives only (no String/Vec).
// Mirrors the #[vil_fault] style prescribed in COMPLIANCE.md §4.
// =============================================================================

use vil_connector_macros::connector_fault;

/// Fault codes specific to the filesystem trigger.
#[connector_fault]
pub enum FsFault {
    /// The watch path could not be registered with the OS watcher.
    /// `path_hash` is the FxHash-32 of the watch path string.
    WatchFailed { path_hash: u32, os_code: u32 },

    /// The watcher event channel closed unexpectedly.
    WatcherChannelClosed,

    /// The trigger task was cancelled.
    TaskCancelled { trigger_id: u64 },

    /// A notify error occurred (the underlying watcher returned an error).
    NotifyError { kind_code: u32 },
}

impl From<FsFault> for vil_trigger_core::TriggerFault {
    fn from(f: FsFault) -> Self {
        match f {
            FsFault::WatchFailed { path_hash, os_code } => {
                vil_trigger_core::TriggerFault::SourceUnavailable {
                    kind_hash: path_hash,
                    reason_code: os_code,
                }
            }
            FsFault::WatcherChannelClosed | FsFault::NotifyError { .. } => {
                vil_trigger_core::TriggerFault::IoError {
                    kind_hash: vil_log::dict::register_str("fs"),
                    os_code: 0,
                }
            }
            FsFault::TaskCancelled { .. } => vil_trigger_core::TriggerFault::IoError {
                kind_hash: vil_log::dict::register_str("fs"),
                os_code: 1,
            },
        }
    }
}
