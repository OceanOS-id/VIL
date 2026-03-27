// =============================================================================
// vil_trigger_webhook — VIL Phase 3 HTTP Webhook Trigger
// =============================================================================
//
// HTTP webhook receiver with HMAC-SHA256 signature verification.
//
// Modules:
//   config  — WebhookConfig (listen_addr, secret, path)
//   source  — WebhookTrigger implements TriggerSource
//   verify  — HMAC-SHA256 verification helper
//   error   — WebhookFault plain enum
//   process — create_trigger convenience constructor
//
// No println!, tracing, or log crate usage — COMPLIANCE.md §8.
// =============================================================================

pub mod config;
pub mod error;
pub mod process;
pub mod source;
pub mod verify;

pub use config::WebhookConfig;
pub use error::WebhookFault;
pub use source::WebhookTrigger;
