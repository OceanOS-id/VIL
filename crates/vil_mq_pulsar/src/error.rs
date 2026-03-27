// =============================================================================
// vil_mq_pulsar::error — PulsarFault (plain enum, u32 fields only)
// =============================================================================

/// Apache Pulsar operation faults.
///
/// Plain enum with u32-only fields per VIL compliance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PulsarFault {
    /// Connection to broker failed.
    ConnectionFailed {
        /// Hash of broker URL.
        url_hash: u32,
        /// Elapsed time in milliseconds.
        elapsed_ms: u32,
    },
    /// Producer creation failed.
    ProducerFailed {
        /// Hash of topic name.
        topic_hash: u32,
    },
    /// Consumer creation failed.
    ConsumerFailed {
        /// Hash of topic name.
        topic_hash: u32,
        /// Hash of subscription name.
        subscription_hash: u32,
    },
    /// Send operation failed.
    SendFailed {
        /// Hash of topic name.
        topic_hash: u32,
        /// Error code from Pulsar.
        error_code: u32,
    },
    /// Receive operation failed.
    ReceiveFailed {
        /// Hash of topic name.
        topic_hash: u32,
    },
    /// Acknowledge failed.
    AckFailed {
        /// Hash of topic name.
        topic_hash: u32,
    },
}
