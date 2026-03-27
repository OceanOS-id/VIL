// =============================================================================
// vil_trigger_fs::config — FsConfig
// =============================================================================
//
// Setup-time configuration (External layout profile — heap types allowed here).
// =============================================================================

/// Which filesystem event kinds the trigger should respond to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FsEventMask {
    /// Fire on file creation.
    pub on_create: bool,
    /// Fire on file modification.
    pub on_modify: bool,
    /// Fire on file deletion.
    pub on_delete: bool,
    /// Fire on file rename.
    pub on_rename: bool,
}

impl FsEventMask {
    /// All event kinds enabled.
    pub fn all() -> Self {
        Self { on_create: true, on_modify: true, on_delete: true, on_rename: true }
    }

    /// Only creation events.
    pub fn create_only() -> Self {
        Self { on_create: true, ..Default::default() }
    }
}

/// Configuration for a `FsTrigger`.
///
/// **Layout profile: External** — used only at initialisation.
#[derive(Debug, Clone)]
pub struct FsConfig {
    /// Numeric identity for this trigger instance.
    pub trigger_id: u64,

    /// Absolute path of the directory (or file) to watch.
    pub watch_path: &'static str,

    /// Optional glob pattern filter (e.g. `"*.csv"`).
    /// If `None`, all events from `watch_path` are forwarded.
    pub pattern: Option<&'static str>,

    /// Debounce window in milliseconds.
    /// Multiple rapid events for the same path are collapsed into one.
    pub debounce_ms: u64,

    /// Watch subdirectories recursively.
    pub recursive: bool,

    /// Which event kinds to forward.
    pub events: FsEventMask,

    /// Capacity of the internal event channel.
    pub channel_capacity: usize,
}

impl FsConfig {
    /// Construct with sensible defaults: no filter, 500 ms debounce, all events.
    pub fn new(trigger_id: u64, watch_path: &'static str) -> Self {
        Self {
            trigger_id,
            watch_path,
            pattern: None,
            debounce_ms: 500,
            recursive: false,
            events: FsEventMask::all(),
            channel_capacity: 256,
        }
    }
}
