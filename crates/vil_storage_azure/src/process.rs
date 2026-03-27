// =============================================================================
// vil_storage_azure — ServiceProcess registration helper
// =============================================================================
//
// Provides a convenience function to create a shared `AzureClient` ready for
// use as a VIL service component.
//
// # Usage in a VilApp context
//
// ```ignore
// use vil_storage_azure::process::create_client;
//
// let client = create_client(config)?;
// ServiceProcess::new("azure")
//     .state(client)
//     .endpoint(...)
// ```

use std::sync::Arc;

use crate::{AzureClient, AzureConfig, AzureFault};

/// Create a shared `AzureClient` wrapped in an `Arc` for multi-owner access.
///
/// `AzureClient::new` is synchronous (credential validation only; no network
/// round-trip at construction time).
pub fn create_client(config: AzureConfig) -> Result<Arc<AzureClient>, AzureFault> {
    let client = AzureClient::new(config)?;
    Ok(Arc::new(client))
}
