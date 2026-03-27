// =============================================================================
// vil_storage_gcs — ServiceProcess registration helper
// =============================================================================
//
// Provides a convenience function to create a shared `GcsClient` ready for
// use as a VIL service component.
//
// # Usage in a VilApp context
//
// ```ignore
// use vil_storage_gcs::process::create_client;
//
// let client = create_client(config).await?;
// ServiceProcess::new("gcs")
//     .state(client)
//     .endpoint(...)
// ```

use std::sync::Arc;

use crate::{GcsClient, GcsConfig, GcsFault};

/// Create a shared `GcsClient` wrapped in an `Arc` for multi-owner access.
///
/// Initialises the GCS client using the given `config` and returns it ready
/// to be stored as `ServiceProcess` state.
pub async fn create_client(config: GcsConfig) -> Result<Arc<GcsClient>, GcsFault> {
    let client = GcsClient::new(config).await?;
    Ok(Arc::new(client))
}
