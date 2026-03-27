// =============================================================================
// vil_trigger_email — VIL Phase 3 IMAP Email Trigger
// =============================================================================
//
// IMAP IDLE push-based email trigger.
//
// Modules:
//   config  — EmailConfig (imap_host, port, username, password, folder)
//   source  — EmailTrigger implements TriggerSource
//   error   — EmailFault plain enum
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

pub use config::EmailConfig;
pub use error::EmailFault;
pub use events::EmailReceived;
pub use source::EmailTrigger;
pub use state::EmailTriggerState;
