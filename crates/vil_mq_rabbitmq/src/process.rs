// =============================================================================
// vil_mq_rabbitmq — ServiceProcess registration helper
// =============================================================================
//
// Provides a convenience function to create a shared `RabbitClient` ready for
// use as a VIL service component.
//
// # Usage in a VilApp context
//
// ```ignore
// use vil_mq_rabbitmq::process::create_client;
//
// let client = create_client(config).await?;
// ServiceProcess::new("rabbitmq")
//     .state(client)
//     .endpoint(...)
// ```

use std::sync::Arc;

use crate::{RabbitClient, RabbitConfig, RabbitFault};

/// Create a shared `RabbitClient` wrapped in an `Arc` for multi-owner access.
///
/// Connects to RabbitMQ using the given `config` and returns the client ready
/// to be stored as `ServiceProcess` state.
pub async fn create_client(config: RabbitConfig) -> Result<Arc<RabbitClient>, RabbitFault> {
    let client = RabbitClient::connect(config).await?;
    Ok(Arc::new(client))
}
