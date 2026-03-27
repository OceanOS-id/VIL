// =============================================================================
// vil_opcua::error — OpcUaFault
// =============================================================================
//
// VIL-compliant plain enum fault type for OPC-UA operations.
// No thiserror, no String fields — COMPLIANCE.md §4 (Semantic Type Compliance).
// All string-derived context is stored as u32 FxHash via register_str().
// =============================================================================

/// Fault type for all OPC-UA client operations.
///
/// All string-derived fields (endpoint, node IDs) are stored as u32 FxHash
/// values registered via `vil_log::dict::register_str()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpcUaFault {
    /// Failed to connect or create a session with the OPC-UA server.
    ConnectionFailed {
        /// FxHash of the endpoint URL.
        endpoint_hash: u32,
        /// Numeric status code from the OPC-UA driver.
        status_code: u32,
    },
    /// A node read operation failed.
    ReadFailed {
        /// FxHash of the node ID string.
        node_hash: u32,
        /// OPC-UA status code.
        status_code: u32,
    },
    /// A node write operation failed.
    WriteFailed {
        /// FxHash of the node ID string.
        node_hash: u32,
        /// OPC-UA status code.
        status_code: u32,
    },
    /// A subscription create/modify operation failed.
    SubscribeFailed {
        /// FxHash of the node ID string.
        node_hash: u32,
        /// OPC-UA status code.
        status_code: u32,
    },
    /// The session was disconnected or expired.
    SessionExpired {
        /// FxHash of the endpoint URL.
        endpoint_hash: u32,
    },
    /// Operation timed out.
    Timeout {
        /// FxHash of the node ID.
        node_hash: u32,
        /// Elapsed time in milliseconds.
        elapsed_ms: u32,
    },
}

impl OpcUaFault {
    /// Return a stable numeric error code for log `error_code` fields.
    pub fn as_error_code(&self) -> u8 {
        match self {
            OpcUaFault::ConnectionFailed { .. } => 1,
            OpcUaFault::ReadFailed { .. }       => 2,
            OpcUaFault::WriteFailed { .. }      => 3,
            OpcUaFault::SubscribeFailed { .. }  => 4,
            OpcUaFault::SessionExpired { .. }   => 5,
            OpcUaFault::Timeout { .. }          => 6,
        }
    }
}
