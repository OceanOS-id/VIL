// =============================================================================
// vil_mq_rabbitmq::events — connector events emitted on Data Lane
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when a message is successfully published to an exchange.
#[connector_event]
pub struct MessagePublished {
    /// FxHash of the exchange name.
    pub exchange_hash: u32,
    /// FxHash of the routing key.
    pub routing_key_hash: u32,
    /// Message payload size in bytes.
    pub message_bytes: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}

/// Emitted when a message is successfully consumed from a queue.
#[connector_event]
pub struct MessageConsumed {
    /// FxHash of the queue name.
    pub queue_hash: u32,
    /// Message payload size in bytes.
    pub message_bytes: u32,
    /// Delivery tag for ack/nack tracking.
    pub delivery_tag: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}

/// Emitted when a consumed message is acknowledged.
#[connector_event]
pub struct MessageAcked {
    /// Delivery tag that was acknowledged.
    pub delivery_tag: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}
