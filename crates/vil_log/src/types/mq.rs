// =============================================================================
// vil_log::types::mq — MqPayload
// =============================================================================
//
// Message queue publish/consume event payload.
// =============================================================================

/// Message queue event payload. Fits in 192 bytes.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MqPayload {
    /// FxHash of the broker name (e.g. "kafka", "nats").
    pub broker_hash: u32,
    /// FxHash of the topic/subject name.
    pub topic_hash: u32,
    /// FxHash of the consumer group name.
    pub group_hash: u32,
    /// Message offset/sequence number.
    pub offset: u64,
    /// Message payload size in bytes.
    pub message_bytes: u32,
    /// End-to-end latency in microseconds (publish → consume).
    pub e2e_latency_us: u32,
    /// Operation: 0=publish 1=consume 2=ack 3=nack 4=dlq
    pub op_type: u8,
    /// Partition/shard number.
    pub partition: u8,
    /// Retry count.
    pub retries: u8,
    /// Compression: 0=none 1=gzip 2=lz4 3=snappy
    pub compression: u8,
    /// Padding.
    pub _pad: [u8; 4],
    /// Inline message header metadata (msgpack).
    pub meta_bytes: [u8; 152],
}

impl Default for MqPayload {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

const _: () = {
    assert!(
        std::mem::size_of::<MqPayload>() <= 192,
        "MqPayload must fit within 192 bytes"
    );
};
