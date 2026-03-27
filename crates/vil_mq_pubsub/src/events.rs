// =============================================================================
// vil_mq_pubsub::events — connector events emitted on Data Lane
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when a message is successfully published to a Pub/Sub topic.
#[connector_event]
pub struct MessagePublished {
    /// FxHash of the topic path.
    pub topic_hash: u32,
    /// Message payload size in bytes.
    pub message_bytes: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}

/// Emitted when a message is successfully received from a Pub/Sub subscription.
#[connector_event]
pub struct MessageReceived {
    /// FxHash of the subscription path.
    pub subscription_hash: u32,
    /// Message payload size in bytes.
    pub message_bytes: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}
