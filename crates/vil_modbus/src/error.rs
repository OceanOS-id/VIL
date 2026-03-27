// =============================================================================
// vil_modbus::error — ModbusFault
// =============================================================================
//
// VIL-compliant plain enum fault type for Modbus operations.
// No thiserror, no String fields — COMPLIANCE.md §4 (Semantic Type Compliance).
// All string-derived context is stored as u32 FxHash via register_str().
// =============================================================================

/// Fault type for all Modbus TCP/RTU client operations.
///
/// All string-derived fields (host, addresses) are stored as u32 FxHash
/// values registered via `vil_log::dict::register_str()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModbusFault {
    /// Failed to establish a TCP connection to the Modbus gateway.
    ConnectionFailed {
        /// FxHash of the host:port string.
        host_hash: u32,
        /// Numeric OS error code.
        reason_code: u32,
    },
    /// A coil read operation failed.
    ReadCoilsFailed {
        /// Starting register address.
        address: u16,
        /// Modbus exception code (if any).
        exception_code: u8,
    },
    /// A holding/input register read failed.
    ReadRegistersFailed {
        /// Starting register address.
        address: u16,
        /// Modbus exception code (if any).
        exception_code: u8,
    },
    /// A single coil write failed.
    WriteCoilFailed {
        /// Coil address.
        address: u16,
        /// Modbus exception code (if any).
        exception_code: u8,
    },
    /// A single register write failed.
    WriteRegisterFailed {
        /// Register address.
        address: u16,
        /// Modbus exception code (if any).
        exception_code: u8,
    },
    /// The request timed out before a response was received.
    Timeout {
        /// FxHash of the host:port string.
        host_hash: u32,
        /// Elapsed time in milliseconds.
        elapsed_ms: u32,
    },
    /// The device returned an unexpected unit ID in the response.
    UnitIdMismatch {
        /// Expected unit ID.
        expected: u8,
        /// Received unit ID.
        received: u8,
    },
}

impl ModbusFault {
    /// Return a stable numeric error code for log `error_code` fields.
    pub fn as_error_code(&self) -> u8 {
        match self {
            ModbusFault::ConnectionFailed { .. }     => 1,
            ModbusFault::ReadCoilsFailed { .. }      => 2,
            ModbusFault::ReadRegistersFailed { .. }  => 3,
            ModbusFault::WriteCoilFailed { .. }      => 4,
            ModbusFault::WriteRegisterFailed { .. }  => 5,
            ModbusFault::Timeout { .. }              => 6,
            ModbusFault::UnitIdMismatch { .. }       => 7,
        }
    }
}
