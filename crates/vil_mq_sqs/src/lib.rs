// =============================================================================
// vil_mq_sqs — VIL AWS SQS/SNS Adapter
// =============================================================================

pub mod config;
pub mod client;
pub mod error;
pub mod events;
pub mod process;
pub mod state;

pub use config::SqsConfig;
pub use client::{SnsClient, SqsClient, SqsMessage};
pub use error::SqsFault;
pub use events::{MessageDeleted, MessageReceived, MessageSent};
pub use state::SqsQueueState;
