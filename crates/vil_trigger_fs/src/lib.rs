// =============================================================================
// vil_trigger_fs — VIL Phase 3 Filesystem / Directory Watcher Trigger
// =============================================================================
//
// Watches a local filesystem path with the `notify` crate (inotify on Linux,
// FSEvents on macOS, ReadDirectoryChangesW on Windows) and fires
// `TriggerEvent` descriptors on matching create/modify/delete events.
//
// # Modules
// - `config`  — FsConfig, FsEventMask, debounce settings
// - `error`   — FsFault (plain enum, no heap)
// - `source`  — FsTrigger: TriggerSource implementation
// - `process` — create_fs_trigger() convenience constructor
//
// # Semantic log
// Every fire emits `mq_log!(Info, MqPayload { ... })` with timing.
// No println!, tracing::info!, log::info! — COMPLIANCE.md §8.
//
// # Tri-Lane mapping
// | Lane    | Direction           | Content                       |
// |---------|---------------------|-------------------------------|
// | Trigger | Outbound → Pipeline | TriggerEvent descriptor       |
// | Data    | N/A                 | (path stored as hash, no SHM) |
// | Control | Inbound ← Pipeline  | Pause / Resume / Stop         |
// =============================================================================

pub mod config;
pub mod error;
pub mod process;
pub mod source;

pub use config::{FsConfig, FsEventMask};
pub use error::FsFault;
pub use process::create_fs_trigger;
pub use source::FsTrigger;
