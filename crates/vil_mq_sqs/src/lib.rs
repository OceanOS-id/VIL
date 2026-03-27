// =============================================================================
// vil_mq_sqs — VIL AWS SQS/SNS Adapter
// =============================================================================

pub mod config;
pub mod client;
pub mod error;
pub mod process;

pub use config::SqsConfig;
pub use client::{SnsClient, SqsClient, SqsMessage};
pub use error::SqsFault;
