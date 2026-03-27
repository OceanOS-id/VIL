// =============================================================================
// vil_db_mongo — ServiceProcess registration helper
// =============================================================================
//
// Provides a convenience function to create a shared `MongoClient` ready for
// use as a VIL service component.
//
// # Usage in a VilApp context
//
// ```ignore
// use vil_db_mongo::process::create_client;
//
// let client = create_client(config).await?;
// ServiceProcess::new("mongo")
//     .state(client)
//     .endpoint(...)
// ```

use std::sync::Arc;

use crate::{MongoClient, MongoConfig, MongoFault};

/// Create a shared `MongoClient` wrapped in an `Arc` for multi-owner access.
///
/// Connects to MongoDB using the given `config` and returns the client ready
/// to be stored as `ServiceProcess` state.
pub async fn create_client(config: MongoConfig) -> Result<Arc<MongoClient>, MongoFault> {
    let client = MongoClient::new(config).await?;
    Ok(Arc::new(client))
}
