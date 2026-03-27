// =============================================================================
// vil_trigger_core — VIL Phase 3 Shared Trigger Infrastructure
// =============================================================================
//
// Provides the foundational trait and types used by all VIL trigger crates.
//
// # Modules
// - `traits`  — TriggerSource async trait
// - `types`   — TriggerEvent (plain struct), TriggerFault (plain enum)
// - `config`  — TriggerConfig, TriggerKind (setup-time, External layout)
// - `process` — create_trigger() ServiceProcess helper
//
// No println!, tracing, or log crate usage — COMPLIANCE.md §8.
// =============================================================================

pub mod config;
pub mod process;
pub mod state;
pub mod traits;
pub mod types;

pub use config::{TriggerConfig, TriggerKind};
pub use process::create_trigger;
pub use state::TriggerState;
pub use traits::{EventCallback, TriggerSource};
pub use types::{TriggerEvent, TriggerFault};
