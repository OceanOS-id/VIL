// =============================================================================
// vil_mq_rabbitmq — VIL RabbitMQ Adapter
// =============================================================================

pub mod config;
pub mod client;
pub mod error;
pub mod process;

pub use config::RabbitConfig;
pub use client::{RabbitClient, RabbitMessage};
pub use error::RabbitFault;
