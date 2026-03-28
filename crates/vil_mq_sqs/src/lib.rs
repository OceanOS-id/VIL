// =============================================================================
// vil_mq_sqs — VIL AWS SQS/SNS Adapter
// =============================================================================

pub mod client;
pub mod config;
pub mod error;
pub mod events;
pub mod process;
pub mod state;

pub use client::{SnsClient, SqsClient, SqsMessage};
pub use config::SqsConfig;
pub use error::SqsFault;
pub use events::{MessageDeleted, MessageReceived, MessageSent};
pub use state::SqsQueueState;
