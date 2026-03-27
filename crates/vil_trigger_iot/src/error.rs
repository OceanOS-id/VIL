// =============================================================================
// vil_trigger_iot::error — IotFault
// =============================================================================
//
// VIL-compliant plain enum fault for MQTT IoT trigger operations.
// No thiserror, no String fields — COMPLIANCE.md §4.
// =============================================================================

/// Fault type for all MQTT IoT trigger operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IotFault {
    /// Failed to connect to the MQTT broker.
    ConnectionFailed {
        /// FxHash of "host:port".
        host_hash: u32,
        /// Numeric connection error code.
        reason_code: u32,
    },
    /// SUBSCRIBE to the topic failed.
    SubscribeFailed {
        /// FxHash of the topic string.
        topic_hash: u32,
        /// MQTT return code.
        return_code: u8,
    },
    /// The MQTT event loop encountered a fatal error.
    EventLoopError {
        /// FxHash of "host:port".
        host_hash: u32,
        /// Error kind code.
        kind_code: u32,
    },
    /// The broker sent an unexpected packet type.
    UnexpectedPacket {
        /// Numeric packet type identifier.
        packet_type: u8,
    },
}

impl IotFault {
    /// Return a stable numeric code for log fields.
    pub fn as_error_code(&self) -> u32 {
        match self {
            IotFault::ConnectionFailed { .. }  => 1,
            IotFault::SubscribeFailed { .. }   => 2,
            IotFault::EventLoopError { .. }    => 3,
            IotFault::UnexpectedPacket { .. }  => 4,
        }
    }
}
