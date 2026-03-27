// =============================================================================
// vil_trigger_fs::error — FsFault
// =============================================================================
//
// Plain enum, primitives only (no String/Vec).
// Mirrors the #[vil_fault] style prescribed in COMPLIANCE.md §4.
// =============================================================================

/// Fault codes specific to the filesystem trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            FsFault::TaskCancelled { .. } => {
                vil_trigger_core::TriggerFault::IoError {
                    kind_hash: vil_log::dict::register_str("fs"),
                    os_code: 1,
                }
            }
        }
    }
}

impl core::fmt::Display for FsFault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::WatchFailed { path_hash, os_code } => {
                write!(f, "FsFault::WatchFailed(path={path_hash:#x}, os={os_code})")
            }
            Self::WatcherChannelClosed => write!(f, "FsFault::WatcherChannelClosed"),
            Self::TaskCancelled { trigger_id } => {
                write!(f, "FsFault::TaskCancelled(id={trigger_id})")
            }
            Self::NotifyError { kind_code } => {
                write!(f, "FsFault::NotifyError(kind={kind_code})")
            }
        }
    }
}
