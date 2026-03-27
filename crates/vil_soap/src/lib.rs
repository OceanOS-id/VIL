// =============================================================================
// vil_soap — VIL SOAP/WSDL Client
// =============================================================================
//
// RPC-style SOAP over HTTP with:
//   - Automatic SOAP envelope build/parse via quick-xml
//   - db_log! auto-emit on every call (op_type=4 CALL) with timing
//   - VIL-compliant plain enum error type (SoapFault)
//   - No println!, tracing::info!, or log::info! — COMPLIANCE.md §8
//
// Thread hint: SoapClient is Send+Sync; uses reqwest connection pooling.
// Add 0 extra log threads (uses caller thread for emit).
// =============================================================================

pub mod config;
pub mod client;
pub mod envelope;
pub mod error;
pub mod process;

pub use config::SoapConfig;
pub use client::SoapClient;
pub use error::SoapFault;
