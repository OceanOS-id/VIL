// =============================================================================
// vil_db_dynamodb — ServiceProcess registration helper
// =============================================================================
//
// Provides a convenience function to create a shared `DynamoClient` ready for
// use as a VIL service component.
//
// # Usage in a VilApp context
//
// ```ignore
// use vil_db_dynamodb::process::create_client;
//
// let client = create_client(config).await?;
// ServiceProcess::new("dynamodb")
//     .state(client)
//     .endpoint(...)
// ```

use std::sync::Arc;

use crate::{DynamoClient, DynamoConfig, DynamoFault};

/// Create a shared `DynamoClient` wrapped in an `Arc` for multi-owner access.
///
/// Connects to DynamoDB using the given `config` and returns the client ready
/// to be stored as `ServiceProcess` state.
pub async fn create_client(config: DynamoConfig) -> Result<Arc<DynamoClient>, DynamoFault> {
    let client = DynamoClient::new(config).await?;
    Ok(Arc::new(client))
}
