// =============================================================================
// vil_mq_pubsub — ServiceProcess registration helper
// =============================================================================
//
// Provides a convenience function to create a shared `PubSubClient` ready for
// use as a VIL service component.
//
// # Usage in a VilApp context
//
// ```ignore
// use vil_mq_pubsub::process::create_client;
//
// let client = create_client(config).await?;
// ServiceProcess::new("pubsub")
//     .state(client)
//     .endpoint(...)
// ```

use std::sync::Arc;

use crate::{PubSubClient, PubSubConfig, PubSubFault};

/// Create a shared `PubSubClient` wrapped in an `Arc` for multi-owner access.
///
/// Connects to Google Cloud Pub/Sub using the given `config` and returns the
/// client ready to be stored as `ServiceProcess` state.
pub async fn create_client(config: PubSubConfig) -> Result<Arc<PubSubClient>, PubSubFault> {
    let client = PubSubClient::new(config).await?;
    Ok(Arc::new(client))
}
