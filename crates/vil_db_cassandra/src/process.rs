// =============================================================================
// vil_db_cassandra — ServiceProcess registration helper
// =============================================================================
//
// Provides a convenience function to create a shared `CassandraClient` ready
// for use as a VIL service component.
//
// # Usage in a VilApp context
//
// ```ignore
// use vil_db_cassandra::process::create_client;
//
// let client = create_client(config).await?;
// ServiceProcess::new("cassandra")
//     .state(client)
//     .endpoint(...)
// ```

use std::sync::Arc;

use crate::{CassandraClient, CassandraConfig, CassandraFault};

/// Create a shared `CassandraClient` wrapped in an `Arc` for multi-owner access.
///
/// Connects to Cassandra/ScyllaDB using the given `config` and returns the
/// client ready to be stored as `ServiceProcess` state.
pub async fn create_client(
    config: CassandraConfig,
) -> Result<Arc<CassandraClient>, CassandraFault> {
    let client = CassandraClient::new(config).await?;
    Ok(Arc::new(client))
}
