// =============================================================================
// vil_ws::state — connector state for ServiceProcess health metrics
// =============================================================================

use vil_connector_macros::connector_state;

/// Live state metrics for the WebSocket server.
#[connector_state]
pub struct WsServerState {
    /// Current number of active connections.
    pub active_connections: u32,
    /// Total connections accepted since startup.
    pub total_connections: u64,
    /// Total messages sent successfully.
    pub messages_sent: u64,
    /// Total messages received.
    pub messages_received: u64,
    /// Total send errors.
    pub send_errors: u64,
}
