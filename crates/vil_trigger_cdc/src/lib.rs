// =============================================================================
// vil_trigger_cdc — VIL Phase 3 CDC Trigger
// =============================================================================
//
// PostgreSQL logical replication CDC trigger.
//
// Modules:
//   config  — CdcConfig (conn_string, slot_name, publication)
//   source  — CdcTrigger implements TriggerSource
//   error   — CdcFault plain enum
//   process — create_trigger convenience constructor
//
// No println!, tracing, or log crate usage — COMPLIANCE.md §8.
// =============================================================================

pub mod config;
pub mod error;
pub mod events;
pub mod process;
pub mod source;
pub mod state;

pub use config::CdcConfig;
pub use error::CdcFault;
pub use events::RowChanged;
pub use source::CdcTrigger;
pub use state::CdcTriggerState;
