// =============================================================================
// vil_db_elastic — VIL Database Plugin: Elasticsearch / OpenSearch
// =============================================================================
//
// Provides an async Elasticsearch client.
//
// # Compliance
//
// - Semantic log:   every operation emits `db_log!` with timing (§8)
// - Error types:    `ElasticFault` plain enum, no `thiserror` (§4, §12)
// - No heap strings on log path: `register_str()` hashes used throughout
// - No `println!`, `tracing::info!`, `eprintln!` in production code
//
// # Quick start
//
// ```rust,ignore
// use vil_db_elastic::{ElasticClient, ElasticConfig};
// use serde_json::json;
//
// #[tokio::main]
// async fn main() {
//     let cfg = ElasticConfig {
//         url: "http://localhost:9200".into(),
//         username: None,
//         password: None,
//     };
//
//     let client = ElasticClient::new(cfg).expect("connect");
//     client.index("my-index", "doc-1", json!({"title": "hello"})).await.expect("index");
//     let doc = client.get("my-index", "doc-1").await.expect("get");
// }
// ```
// =============================================================================

pub mod client;
pub mod config;
pub mod error;
pub mod events;
pub mod process;
pub mod state;

pub use client::{ElasticClient, IndexResult, SearchResult};
pub use config::ElasticConfig;
pub use error::ElasticFault;
pub use events::{ElasticDocumentIndexed, ElasticSearchExecuted};
pub use state::ElasticClientState;
