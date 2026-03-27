// =============================================================================
// vil_soap — ServiceProcess registration helper
// =============================================================================
//
// Provides a convenience function to create a shared `SoapClient` ready for
// use as a VIL service component.
//
// # Usage in a VilApp context
//
// ```ignore
// use vil_soap::process::create_client;
//
// let client = create_client(config)?;
// ServiceProcess::new("soap")
//     .state(client)
//     .endpoint(...)
// ```

use std::sync::Arc;

use crate::{SoapClient, SoapConfig, SoapFault};

/// Create a shared `SoapClient` wrapped in an `Arc` for multi-owner access.
///
/// `SoapClient::new` is synchronous (HTTP client setup only).
pub fn create_client(config: SoapConfig) -> Result<Arc<SoapClient>, SoapFault> {
    let client = SoapClient::new(config)?;
    Ok(Arc::new(client))
}
