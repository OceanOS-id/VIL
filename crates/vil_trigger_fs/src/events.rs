// =============================================================================
// vil_trigger_fs::events — connector events emitted on Data Lane
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when a filesystem change is detected on a watched path.
#[connector_event]
pub struct FileChanged {
    /// FxHash of the watch path string.
    pub path_hash: u32,
    /// Event kind: 0=create, 1=modify, 2=delete, 3=rename, 255=other.
    pub event_kind: u8,
    /// Debounce window identifier (groups rapid consecutive events).
    pub debounce_id: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}
