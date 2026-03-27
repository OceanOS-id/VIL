// =============================================================================
// vil_mq_pubsub::state — connector state for ServiceProcess health metrics
// =============================================================================

use vil_connector_macros::connector_state;

/// Live state metrics for the Google Cloud Pub/Sub client.
#[connector_state]
pub struct PubSubState {
    /// Total messages published successfully.
    pub messages_published: u64,
    /// Total messages received successfully.
    pub messages_received: u64,
    /// Total acknowledgements sent.
    pub messages_acked: u64,
    /// Total publish/receive errors.
    pub pubsub_errors: u64,
}
