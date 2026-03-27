// =============================================================================
// vil_ws::events — connector events emitted on Data Lane
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when a message is successfully sent to a WebSocket client.
#[connector_event]
pub struct MessageSent {
    /// FxHash of the room/topic name.
    pub room_hash: u32,
    /// Message payload size in bytes.
    pub message_bytes: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}

/// Emitted when a message is received from a WebSocket client.
#[connector_event]
pub struct MessageReceived {
    /// FxHash of the room/topic name.
    pub room_hash: u32,
    /// Message payload size in bytes.
    pub message_bytes: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}

/// Emitted when a new WebSocket client connects.
#[connector_event]
pub struct ClientConnected {
    /// FxHash of the remote address string.
    pub addr_hash: u32,
    /// Current total connection count after this connection.
    pub total_connections: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}

/// Emitted when a WebSocket client disconnects.
#[connector_event]
pub struct ClientDisconnected {
    /// FxHash of the remote address string.
    pub addr_hash: u32,
    /// Current total connection count after this disconnection.
    pub total_connections: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}
