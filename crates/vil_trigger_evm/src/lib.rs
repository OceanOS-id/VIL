// =============================================================================
// vil_trigger_evm — VIL Phase 3 EVM Blockchain Event Trigger
// =============================================================================
//
// Ethereum JSON-RPC log subscription trigger using alloy.
//
// Modules:
//   config  — EvmConfig (rpc_url, contract_address, event_signature)
//   source  — EvmTrigger implements TriggerSource
//   error   — EvmFault plain enum
//   process — create_trigger convenience constructor
//
// No println!, tracing, or log crate usage — COMPLIANCE.md §8.
// =============================================================================

pub mod config;
pub mod error;
pub mod process;
pub mod source;

pub use config::EvmConfig;
pub use error::EvmFault;
pub use source::EvmTrigger;
