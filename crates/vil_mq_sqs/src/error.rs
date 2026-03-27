// =============================================================================
// vil_mq_sqs::error — SqsFault (plain enum, u32 fields only)
// =============================================================================

/// AWS SQS/SNS operation faults.
///
/// Plain enum with u32-only fields per VIL compliance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqsFault {
    /// Failed to load AWS config.
    ConfigLoadFailed {
        /// Hash of region string.
        region_hash: u32,
    },
    /// Failed to send message to SQS.
    SendFailed {
        /// Hash of queue URL.
        queue_hash: u32,
        /// AWS error code (0 = unknown).
        error_code: u32,
    },
    /// Failed to receive messages from SQS.
    ReceiveFailed {
        /// Hash of queue URL.
        queue_hash: u32,
    },
    /// Failed to delete message from SQS.
    DeleteFailed {
        /// Hash of queue URL.
        queue_hash: u32,
        /// Error code.
        error_code: u32,
    },
    /// SNS publish failed.
    SnsPublishFailed {
        /// Hash of topic ARN.
        topic_hash: u32,
        /// Error code.
        error_code: u32,
    },
    /// Message body was empty or invalid.
    InvalidMessage,
}
