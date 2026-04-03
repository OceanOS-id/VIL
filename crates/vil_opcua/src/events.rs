// =============================================================================
// vil_opcua::events — connector events emitted on Data Lane
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when a node value is successfully read.
#[connector_event]
pub struct NodeRead {
    /// FxHash of the node ID string.
    pub node_hash: u32,
    /// OPC-UA status code of the read result.
    pub status_code: u32,
    /// Round-trip latency in microseconds.
    pub latency_ns: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}

/// Emitted when a node value is successfully written.
#[connector_event]
pub struct NodeWritten {
    /// FxHash of the node ID string.
    pub node_hash: u32,
    /// OPC-UA status code of the write result.
    pub status_code: u32,
    /// Round-trip latency in microseconds.
    pub latency_ns: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}

/// Emitted when a subscription value notification arrives.
#[connector_event]
pub struct ValueSubscribed {
    /// FxHash of the node ID string.
    pub node_hash: u32,
    /// Subscription ID assigned by the server.
    pub subscription_id: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}
