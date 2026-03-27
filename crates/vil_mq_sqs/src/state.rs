// =============================================================================
// vil_mq_sqs::state — connector state for ServiceProcess health metrics
// =============================================================================

use vil_connector_macros::connector_state;

/// Live state metrics for the SQS queue client.
#[connector_state]
pub struct SqsQueueState {
    /// Total messages sent successfully.
    pub messages_sent: u64,
    /// Total messages received successfully.
    pub messages_received: u64,
    /// Total messages deleted successfully.
    pub messages_deleted: u64,
    /// Total send/receive/delete errors.
    pub queue_errors: u64,
}
