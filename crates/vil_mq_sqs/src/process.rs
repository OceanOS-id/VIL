// =============================================================================
// vil_mq_sqs — ServiceProcess registration helper
// =============================================================================
//
// Provides a convenience function to create a shared `SqsClient` ready for
// use as a VIL service component.
//
// # Usage in a VilApp context
//
// ```ignore
// use vil_mq_sqs::process::create_client;
//
// let client = create_client(config).await?;
// ServiceProcess::new("sqs")
//     .state(client)
//     .endpoint(...)
// ```

use std::sync::Arc;

use crate::{SqsClient, SqsConfig, SqsFault};

/// Create a shared `SqsClient` wrapped in an `Arc` for multi-owner access.
///
/// Initialises the SQS client using the given `config` and returns it ready
/// to be stored as `ServiceProcess` state.
pub async fn create_client(config: SqsConfig) -> Result<Arc<SqsClient>, SqsFault> {
    let client = SqsClient::from_config(config).await?;
    Ok(Arc::new(client))
}
