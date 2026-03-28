// =============================================================================
// vil_trigger_cdc::error — CdcFault
// =============================================================================
//
// VIL-compliant fault type for CDC operations.
// No thiserror, no String fields — COMPLIANCE.md §4.
// All string context stored as u32 FxHash via register_str().
// =============================================================================

use vil_connector_macros::connector_fault;

/// Fault type for all PostgreSQL CDC trigger operations.
#[connector_fault]
pub enum CdcFault {
    /// Failed to connect to PostgreSQL.
    ConnectionFailed {
        /// FxHash of the connection string.
        conn_hash: u32,
        /// OS or libpq error code.
        reason_code: u32,
    },
    /// Failed to start logical replication on the given slot.
    ReplicationStartFailed {
        /// FxHash of the slot name.
        slot_hash: u32,
        /// Error code returned by PostgreSQL.
        pg_error_code: u32,
    },
    /// Received an unrecognised replication message type.
    UnknownMessage {
        /// Raw message type byte.
        msg_type: u8,
    },
    /// The replication slot was not found on the server.
    SlotNotFound {
        /// FxHash of the slot name.
        slot_hash: u32,
    },
    /// The publication was not found on the server.
    PublicationNotFound {
        /// FxHash of the publication name.
        pub_hash: u32,
    },
    /// The replication stream closed unexpectedly.
    StreamClosed {
        /// FxHash of the slot name.
        slot_hash: u32,
    },
    /// A keepalive write back to the server failed.
    KeepaliveFailed {
        /// OS error code.
        os_code: u32,
    },
}

impl CdcFault {
    /// Return a stable numeric error code for log fields.
    pub fn as_error_code(&self) -> u32 {
        match self {
            CdcFault::ConnectionFailed { .. } => 1,
            CdcFault::ReplicationStartFailed { .. } => 2,
            CdcFault::UnknownMessage { .. } => 3,
            CdcFault::SlotNotFound { .. } => 4,
            CdcFault::PublicationNotFound { .. } => 5,
            CdcFault::StreamClosed { .. } => 6,
            CdcFault::KeepaliveFailed { .. } => 7,
        }
    }
}
