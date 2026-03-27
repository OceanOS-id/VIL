// =============================================================================
// vil_mq_pulsar — ServiceProcess registration helper
// =============================================================================
//
// Provides a convenience function to create a shared `PulsarClient` ready for
// use as a VIL service component.
//
// # Usage in a VilApp context
//
// ```ignore
// use vil_mq_pulsar::process::create_client;
//
// let client = create_client(config).await?;
// ServiceProcess::new("pulsar")
//     .state(client)
//     .endpoint(...)
// ```

use std::sync::Arc;

use crate::{PulsarClient, PulsarConfig, PulsarFault};

/// Create a shared `PulsarClient` wrapped in an `Arc` for multi-owner access.
///
/// Connects to Apache Pulsar using the given `config` and returns the client
/// ready to be stored as `ServiceProcess` state.
pub async fn create_client(config: PulsarConfig) -> Result<Arc<PulsarClient>, PulsarFault> {
    let client = PulsarClient::connect(config).await?;
    Ok(Arc::new(client))
}
