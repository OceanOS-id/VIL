// =============================================================================
// vil_ws::config — WsConfig
// =============================================================================
//
// Configuration for the VIL WebSocket server.
// External layout profile acceptable for setup-time data (COMPLIANCE.md §4).
// =============================================================================

/// Configuration for the VIL WebSocket server.
///
/// # Example YAML
/// ```yaml
/// websocket:
///   addr: "0.0.0.0:9000"
///   max_connections: 10000
///   max_message_bytes: 65536
/// ```
#[derive(Debug, Clone)]
pub struct WsConfig {
    /// Bind address (e.g. "0.0.0.0:9000").
    pub addr: String,
    /// Maximum number of concurrent WebSocket connections.
    pub max_connections: usize,
    /// Maximum incoming message size in bytes.
    pub max_message_bytes: usize,
}

impl WsConfig {
    /// Construct a new `WsConfig` with defaults.
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            addr: addr.into(),
            max_connections: 10_000,
            max_message_bytes: 65_536,
        }
    }

    /// Override the maximum connection limit.
    pub fn with_max_connections(mut self, n: usize) -> Self {
        self.max_connections = n;
        self
    }

    /// Override the maximum message size.
    pub fn with_max_message_bytes(mut self, n: usize) -> Self {
        self.max_message_bytes = n;
        self
    }
}
