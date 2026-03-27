// =============================================================================
// vil_mq_pubsub::error — PubSubFault (plain enum, u32 fields only)
// =============================================================================

/// Google Cloud Pub/Sub operation faults.
///
/// Plain enum with u32-only fields per VIL compliance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PubSubFault {
    /// Client initialization failed.
    ClientInitFailed {
        /// Hash of project ID.
        project_hash: u32,
    },
    /// Publisher creation failed.
    PublisherFailed {
        /// Hash of topic path.
        topic_hash: u32,
    },
    /// Subscriber creation failed.
    SubscriberFailed {
        /// Hash of subscription path.
        subscription_hash: u32,
    },
    /// Publish operation failed.
    PublishFailed {
        /// Hash of topic path.
        topic_hash: u32,
        /// gRPC status code (0 = unknown).
        status_code: u32,
    },
    /// Receive operation failed.
    ReceiveFailed {
        /// Hash of subscription path.
        subscription_hash: u32,
    },
    /// Acknowledge failed.
    AckFailed {
        /// Hash of subscription path.
        subscription_hash: u32,
    },
}
