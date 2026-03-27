// =============================================================================
// vil_db_clickhouse — ServiceProcess registration helper
// =============================================================================
//
// Provides a convenience function to create a shared `ChClient` ready for
// use as a VIL service component.
//
// # Usage in a VilApp context
//
// ```ignore
// use vil_db_clickhouse::process::create_client;
//
// let client = create_client(config);
// ServiceProcess::new("clickhouse")
//     .state(client)
//     .endpoint(...)
// ```

use std::sync::Arc;

use crate::{ChClient, ClickHouseConfig};

/// Create a shared `ChClient` wrapped in an `Arc` for multi-owner access.
///
/// `ChClient::new` is synchronous so no `.await` is required.
pub fn create_client(config: ClickHouseConfig) -> Arc<ChClient> {
    Arc::new(ChClient::new(config))
}
