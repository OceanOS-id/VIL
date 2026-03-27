// =============================================================================
// vil_mq_rabbitmq::state — connector state for ServiceProcess health metrics
// =============================================================================

use vil_connector_macros::connector_state;

/// Live state metrics for the RabbitMQ channel.
#[connector_state]
pub struct RabbitChannelState {
    /// Total messages published successfully.
    pub messages_published: u64,
    /// Total messages consumed successfully.
    pub messages_consumed: u64,
    /// Total messages acknowledged.
    pub messages_acked: u64,
    /// Total channel-level errors encountered.
    pub channel_errors: u64,
}
