// =============================================================================
// vil_mq_rabbitmq — VIL RabbitMQ Adapter
// =============================================================================

pub mod config;
pub mod client;
pub mod error;
pub mod events;
pub mod process;
pub mod state;

pub use config::RabbitConfig;
pub use client::{RabbitClient, RabbitMessage};
pub use error::RabbitFault;
pub use events::{MessageAcked, MessageConsumed, MessagePublished};
pub use state::RabbitChannelState;
