// =============================================================================
// vil_mq_rabbitmq::error — RabbitFault
// =============================================================================

use vil_connector_macros::connector_fault;

/// RabbitMQ operation faults.
///
/// Plain enum with u32-only fields per VIL compliance (no String/heap types).
/// Hashes are produced via `vil_log::dict::register_str` at the call site.
#[connector_fault]
pub enum RabbitFault {
    /// Connection to broker failed.
    ConnectionFailed {
        /// Hash of the URI (via register_str).
        uri_hash: u32,
        /// Elapsed time in milliseconds.
        elapsed_ms: u32,
    },
    /// Channel creation failed.
    ChannelFailed {
        /// Error code from lapin.
        code: u32,
    },
    /// Publish operation failed.
    PublishFailed {
        /// Hash of exchange name.
        exchange_hash: u32,
        /// Hash of routing key.
        routing_key_hash: u32,
    },
    /// Consume setup failed.
    ConsumeFailed {
        /// Hash of queue name.
        queue_hash: u32,
    },
    /// Ack/Nack failed.
    AckFailed {
        /// Delivery tag that failed.
        delivery_tag: u32,
    },
    /// Not connected.
    NotConnected,
}
