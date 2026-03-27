// =============================================================================
// vil_mq_sqs::events — connector events emitted on Data Lane
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when a message is successfully sent to SQS.
#[connector_event]
pub struct MessageSent {
    /// FxHash of the queue URL.
    pub queue_hash: u32,
    /// Message payload size in bytes.
    pub message_bytes: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}

/// Emitted when messages are successfully received from SQS.
#[connector_event]
pub struct MessageReceived {
    /// FxHash of the queue URL.
    pub queue_hash: u32,
    /// Number of messages received in this batch.
    pub message_count: u32,
    /// Total payload size in bytes across all messages.
    pub total_bytes: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}

/// Emitted when a message is successfully deleted from SQS.
#[connector_event]
pub struct MessageDeleted {
    /// FxHash of the queue URL.
    pub queue_hash: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}
