// =============================================================================
// vil_db_neo4j — VIL Database Plugin: Neo4j Graph Database
// =============================================================================
//
// Neo4j graph client wrapper with VIL semantic log integration.
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
// use vil_db_neo4j::{Neo4jClient, Neo4jConfig};
//
// #[tokio::main]
// async fn main() {
//     let config = Neo4jConfig::new("bolt://localhost:7687", "neo4j", "password");
//     let client = Neo4jClient::new(config).await.expect("connect");
//
//     // match_query collects all rows from a MATCH statement
//     let rows = client
//         .match_query("MATCH (n:Person) RETURN n LIMIT 10")
//         .await
//         .unwrap();
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
pub use client::Neo4jClient;
pub use config::Neo4jConfig;
pub use error::Neo4jFault;
pub use events::{Neo4jNodeCreated, Neo4jRelationCreated};
pub use state::Neo4jSessionState;
pub use types::Neo4jResult;
