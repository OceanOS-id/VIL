// =============================================================================
// vil_modbus — VIL Modbus TCP/RTU Client
// =============================================================================
//
// Industrial Modbus TCP/RTU client with:
//   - db_log! auto-emit on every read/write operation with timing
//   - VIL-compliant plain enum error type (ModbusFault)
//   - No println!, tracing::info!, or log::info! — COMPLIANCE.md §8
//
// Thread hint: ModbusClient is async; uses tokio-modbus internally.
// No extra threads spawned beyond the tokio runtime.
// =============================================================================

pub mod config;
pub mod client;
pub mod error;
pub mod events;
pub mod process;
pub mod state;

pub use config::ModbusConfig;
pub use client::ModbusClient;
pub use error::ModbusFault;
pub use events::{RegisterRead, RegisterWritten};
pub use state::ModbusClientState;
