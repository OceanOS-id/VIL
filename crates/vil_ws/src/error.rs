// =============================================================================
// vil_ws::error — WsFault
// =============================================================================
//
// VIL-compliant fault type for WebSocket server operations.
// No thiserror, no String fields — COMPLIANCE.md §4 (Semantic Type Compliance).
// All string-derived context is stored as u32 FxHash via register_str().
// =============================================================================

use vil_connector_macros::connector_fault;

/// Fault type for all WebSocket server operations.
///
/// All string-derived fields (addr, room names) are stored as u32 FxHash
/// values registered via `vil_log::dict::register_str()`.
#[connector_fault]
pub enum WsFault {
    /// The server failed to bind to the configured address.
    BindFailed {
        /// FxHash of the bind address string.
        addr_hash: u32,
        /// OS error code.
        reason_code: u32,
    },
    /// The WebSocket handshake failed for a new connection.
    HandshakeFailed {
        /// Numeric reason code from tungstenite.
        reason_code: u32,
    },
    /// Maximum connection limit has been reached.
    ConnectionLimitReached {
        /// Current connection count at time of rejection.
        current_count: u32,
        /// Configured maximum.
        max_connections: u32,
    },
    /// A message send to a client failed.
    SendFailed {
        /// FxHash of the topic/room name.
        topic_hash: u32,
        /// Numeric tungstenite error code.
        reason_code: u32,
    },
    /// A broadcast to all clients in a room failed for some recipients.
    BroadcastPartialFail {
        /// FxHash of the room name.
        room_hash: u32,
        /// Number of recipients that failed.
        failed_count: u32,
    },
    /// The room was not found.
    RoomNotFound {
        /// FxHash of the room name.
        room_hash: u32,
    },
    /// An incoming message exceeded the configured size limit.
    MessageTooLarge {
        /// Received message size in bytes.
        received_bytes: u32,
        /// Maximum allowed bytes.
        max_bytes: u32,
    },
}

impl WsFault {
    /// Return a stable numeric error code for log `error_code` fields.
    pub fn as_error_code(&self) -> u8 {
        match self {
            WsFault::BindFailed { .. } => 1,
            WsFault::HandshakeFailed { .. } => 2,
            WsFault::ConnectionLimitReached { .. } => 3,
            WsFault::SendFailed { .. } => 4,
            WsFault::BroadcastPartialFail { .. } => 5,
            WsFault::RoomNotFound { .. } => 6,
            WsFault::MessageTooLarge { .. } => 7,
        }
    }
}
