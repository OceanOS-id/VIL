// =============================================================================
// vil_opcua — ServiceProcess registration helper
// =============================================================================
//
// Provides a convenience function to create a shared `OpcUaClient` ready for
// use as a VIL service component.
//
// # Usage in a VilApp context
//
// ```ignore
// use vil_opcua::process::create_client;
//
// let client = create_client(config)?;
// ServiceProcess::new("opcua")
//     .state(client)
//     .endpoint(...)
// ```

use std::sync::Arc;

use crate::{OpcUaClient, OpcUaConfig, OpcUaFault};

/// Create a shared `OpcUaClient` wrapped in an `Arc` for multi-owner access.
///
/// `OpcUaClient::connect` is synchronous (session setup happens lazily).
pub fn create_client(config: OpcUaConfig) -> Result<Arc<OpcUaClient>, OpcUaFault> {
    let client = OpcUaClient::connect(config)?;
    Ok(Arc::new(client))
}
