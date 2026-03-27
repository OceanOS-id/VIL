// =============================================================================
// vil_ws — ServiceProcess registration helper
// =============================================================================
//
// Provides a convenience function to create a shared `WsServer` ready for
// use as a VIL service component.
//
// # Usage in a VilApp context
//
// ```ignore
// use vil_ws::process::create_server;
//
// let server = create_server(config);
// ServiceProcess::new("ws")
//     .state(server)
//     .endpoint(...)
// ```

use std::sync::Arc;

use crate::{WsConfig, WsServer};

/// Create a shared `WsServer` wrapped in an `Arc` for multi-owner access.
///
/// `WsServer::new` is synchronous. Call `server.run().await` separately to
/// start accepting connections.
pub fn create_server(config: WsConfig) -> Arc<WsServer> {
    Arc::new(WsServer::new(config))
}
