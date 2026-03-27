// =============================================================================
// vil_storage_s3 — ServiceProcess registration helper
// =============================================================================
//
// Provides a convenience function to create a shared `S3Client` ready for
// use as a VIL service component.
//
// # Usage in a VilApp context
//
// ```ignore
// use vil_storage_s3::process::create_client;
//
// let client = create_client(config).await?;
// ServiceProcess::new("s3")
//     .state(client)
//     .endpoint(...)
// ```

use std::sync::Arc;

use crate::{S3Client, S3Config, S3Fault};

/// Create a shared `S3Client` wrapped in an `Arc` for multi-owner access.
///
/// Initialises the S3 client using the given `config` and returns it ready
/// to be stored as `ServiceProcess` state.
pub async fn create_client(config: S3Config) -> Result<Arc<S3Client>, S3Fault> {
    let client = S3Client::new(config).await?;
    Ok(Arc::new(client))
}
