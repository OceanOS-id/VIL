// =============================================================================
// vil_opcua — VIL OPC-UA Client
// =============================================================================
//
// Industrial OPC-UA protocol client with:
//   - db_log! auto-emit on every read/write/subscribe (op_type=0/2/4)
//   - VIL-compliant plain enum error type (OpcUaFault)
//   - No println!, tracing::info!, or log::info! — COMPLIANCE.md §8
//
// Thread hint: OpcUaClient internally spawns OPC-UA session threads.
// Add 2 to your LogConfig.threads for optimal log ring sizing.
// =============================================================================

pub mod config;
pub mod client;
pub mod error;
pub mod process;

pub use config::OpcUaConfig;
pub use client::OpcUaClient;
pub use error::OpcUaFault;
