// =============================================================================
// vil_db_elastic — ServiceProcess registration helper
// =============================================================================
//
// Provides a convenience function to create a shared `ElasticClient` ready
// for use as a VIL service component.
//
// # Usage in a VilApp context
//
// ```ignore
// use vil_db_elastic::process::create_client;
//
// let client = create_client(config)?;
// ServiceProcess::new("elastic")
//     .state(client)
//     .endpoint(...)
// ```

use std::sync::Arc;

use crate::{ElasticClient, ElasticConfig, ElasticFault};

/// Create a shared `ElasticClient` wrapped in an `Arc` for multi-owner access.
///
/// `ElasticClient::new` is synchronous (HTTP client setup only; no network
/// round-trip at construction time).
pub fn create_client(config: ElasticConfig) -> Result<Arc<ElasticClient>, ElasticFault> {
    let client = ElasticClient::new(config)?;
    Ok(Arc::new(client))
}
