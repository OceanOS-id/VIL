// =============================================================================
// vil_mq_pulsar::state — connector state for ServiceProcess health metrics
// =============================================================================

use vil_connector_macros::connector_state;

/// Live state metrics for the Pulsar producer/consumer.
#[connector_state]
pub struct PulsarProducerState {
    /// Total messages sent successfully.
    pub messages_sent: u64,
    /// Total messages received successfully.
    pub messages_received: u64,
    /// Total send/receive errors.
    pub producer_errors: u64,
    /// Total acknowledgement failures.
    pub ack_failures: u64,
}
