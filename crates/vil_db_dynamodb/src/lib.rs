// =============================================================================
// vil_db_dynamodb — VIL Database Plugin: AWS DynamoDB
// =============================================================================
//
// DynamoDB client wrapper with VIL semantic log integration.
//
// # Compliance
//
// - COMPLIANCE.md §8  (Semantic Log):  `vil_log` dependency, `db_log!` on every
//   operation, no println!/tracing, `register_str()` for all hash fields.
// - COMPLIANCE.md §11 (Crate Structure): config / client / ops / error / types
//   module layout.
//
// # Quick Start
//
// ```rust,no_run
// use vil_db_dynamodb::{DynamoClient, DynamoConfig};
// use aws_sdk_dynamodb::types::AttributeValue;
// use std::collections::HashMap;
//
// #[tokio::main]
// async fn main() {
//     let config = DynamoConfig::new("us-east-1");
//     let client = DynamoClient::new(config).await.expect("connect");
//
//     let mut key = HashMap::new();
//     key.insert("id".to_string(), AttributeValue::S("user#123".to_string()));
//
//     let item = client.get_item("users", key).await.unwrap();
// }
// ```
// =============================================================================

pub mod client;
pub mod config;
pub mod error;
pub mod events;
pub mod ops;
pub mod process;
pub mod state;
pub mod types;

// Flat re-exports
pub use client::DynamoClient;
pub use config::DynamoConfig;
pub use error::DynamoFault;
pub use events::{DynamoItemDeleted, DynamoItemPut};
pub use state::DynamoClientState;
pub use types::DynamoResult;
