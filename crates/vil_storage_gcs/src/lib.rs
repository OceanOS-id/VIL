// =============================================================================
// vil_storage_gcs — VIL Storage Plugin: Google Cloud Storage
// =============================================================================
//
// Provides an async GCS client for Google Cloud Storage buckets.
//
// # Compliance
//
// - Semantic log:   every operation emits `db_log!` with timing (§8)
// - Error types:    `GcsFault` plain enum, no `thiserror` (§4, §12)
// - No heap strings on log path: `register_str()` hashes used throughout
// - No `println!`, `tracing::info!`, `eprintln!` in production code
//
// # Quick start
//
// ```rust,ignore
// use vil_storage_gcs::{GcsClient, GcsConfig};
//
// #[tokio::main]
// async fn main() {
//     let cfg = GcsConfig {
//         bucket: "my-bucket".into(),
//         credentials_path: None,
//     };
//
//     let client = GcsClient::new(cfg).await.expect("connect");
//     client.upload("hello.txt", bytes::Bytes::from("hello world")).await.expect("upload");
//     let data = client.download("hello.txt").await.expect("download");
//     assert_eq!(&data[..], b"hello world");
// }
// ```
// =============================================================================

pub mod client;
pub mod config;
pub mod error;
pub mod events;
pub mod process;
pub mod state;

pub use client::{GcsBlobMeta, GcsClient, GcsUploadResult};
pub use config::GcsConfig;
pub use error::GcsFault;
pub use events::{GcsObjectCreated, GcsObjectDeleted};
pub use state::GcsClientState;
