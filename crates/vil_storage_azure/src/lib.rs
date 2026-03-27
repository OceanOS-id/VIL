// =============================================================================
// vil_storage_azure — VIL Storage Plugin: Azure Blob Storage
// =============================================================================
//
// Provides an async Azure Blob Storage client.
//
// # Compliance
//
// - Semantic log:   every operation emits `db_log!` with timing (§8)
// - Error types:    `AzureFault` plain enum, no `thiserror` (§4, §12)
// - No heap strings on log path: `register_str()` hashes used throughout
// - No `println!`, `tracing::info!`, `eprintln!` in production code
//
// # Quick start
//
// ```rust,ignore
// use vil_storage_azure::{AzureClient, AzureConfig};
//
// #[tokio::main]
// async fn main() {
//     let cfg = AzureConfig {
//         account: "myaccount".into(),
//         access_key: "base64key==".into(),
//         container: "my-container".into(),
//     };
//
//     let client = AzureClient::new(cfg).expect("connect");
//     client.upload_blob("hello.txt", bytes::Bytes::from("hello world")).await.expect("upload");
//     let data = client.download_blob("hello.txt").await.expect("download");
//     assert_eq!(&data[..], b"hello world");
// }
// ```
// =============================================================================

pub mod client;
pub mod config;
pub mod error;
pub mod process;

pub use client::{AzureBlobMeta, AzureClient, AzureUploadResult};
pub use config::AzureConfig;
pub use error::AzureFault;
