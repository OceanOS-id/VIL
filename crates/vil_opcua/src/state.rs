// =============================================================================
// vil_opcua::state — connector state for ServiceProcess health metrics
// =============================================================================

use vil_connector_macros::connector_state;

/// Live state metrics for the OPC-UA session.
#[connector_state]
pub struct OpcUaSessionState {
    /// Total node read operations completed.
    pub nodes_read: u64,
    /// Total node write operations completed.
    pub nodes_written: u64,
    /// Total subscription notifications received.
    pub subscriptions_received: u64,
    /// Total OPC-UA errors encountered.
    pub session_errors: u64,
}
