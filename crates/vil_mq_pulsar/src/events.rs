// =============================================================================
// vil_mq_pulsar::events — connector events emitted on Data Lane
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when a message is successfully sent to a Pulsar topic.
#[connector_event]
pub struct MessageSent {
    /// FxHash of the topic name.
    pub topic_hash: u32,
    /// Message payload size in bytes.
    pub message_bytes: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}

/// Emitted when a message is successfully received from a Pulsar topic.
#[connector_event]
pub struct MessageReceived {
    /// FxHash of the topic name.
    pub topic_hash: u32,
    /// FxHash of the subscription name.
    pub subscription_hash: u32,
    /// Message payload size in bytes.
    pub message_bytes: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}
