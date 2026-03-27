// =============================================================================
// vil_db_neo4j — ServiceProcess registration helper
// =============================================================================
//
// Provides a convenience function to create a shared `Neo4jClient` ready for
// use as a VIL service component.
//
// # Usage in a VilApp context
//
// ```ignore
// use vil_db_neo4j::process::create_client;
//
// let client = create_client(config).await?;
// ServiceProcess::new("neo4j")
//     .state(client)
//     .endpoint(...)
// ```

use std::sync::Arc;

use crate::{Neo4jClient, Neo4jConfig, Neo4jFault};

/// Create a shared `Neo4jClient` wrapped in an `Arc` for multi-owner access.
///
/// Connects to Neo4j using the given `config` and returns the client ready to
/// be stored as `ServiceProcess` state.
pub async fn create_client(config: Neo4jConfig) -> Result<Arc<Neo4jClient>, Neo4jFault> {
    let client = Neo4jClient::new(config).await?;
    Ok(Arc::new(client))
}
