// =============================================================================
// vil_modbus — ServiceProcess registration helper
// =============================================================================
//
// Provides a convenience function to create a shared `ModbusClient` ready for
// use as a VIL service component.
//
// # Usage in a VilApp context
//
// ```ignore
// use vil_modbus::process::create_client;
//
// let client = create_client(config).await?;
// ServiceProcess::new("modbus")
//     .state(client)
//     .endpoint(...)
// ```

use std::sync::Arc;

use crate::{ModbusClient, ModbusConfig, ModbusFault};

/// Create a shared `ModbusClient` wrapped in an `Arc` for multi-owner access.
///
/// Connects to the Modbus TCP/RTU device using the given `config` and returns
/// the client ready to be stored as `ServiceProcess` state.
pub async fn create_client(config: ModbusConfig) -> Result<Arc<ModbusClient>, ModbusFault> {
    let client = ModbusClient::connect(config).await?;
    Ok(Arc::new(client))
}
