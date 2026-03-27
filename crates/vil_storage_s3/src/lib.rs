// =============================================================================
// vil_storage_s3 — VIL Storage Plugin: S3-compatible object storage
// =============================================================================
//
// Provides an async S3 client compatible with AWS S3, MinIO, Cloudflare R2,
// DigitalOcean Spaces, and any service that implements the AWS S3 API.
//
// # Compliance
//
// - Semantic log:   every operation emits `db_log!` with timing (§8)
// - Error types:    `S3Fault` plain enum, no `thiserror` (§4, §12)
// - No heap strings on log path: `register_str()` hashes used throughout
// - No `println!`, `tracing::info!`, `eprintln!` in production code
//
// # Quick start
//
// ```rust,ignore
// use vil_storage_s3::{S3Client, S3Config};
//
// #[tokio::main]
// async fn main() {
//     let cfg = S3Config {
//         endpoint: Some("http://localhost:9000".into()),
//         region: "us-east-1".into(),
//         access_key: Some("minioadmin".into()),
//         secret_key: Some("minioadmin".into()),
//         bucket: "my-bucket".into(),
//         path_style: true,
//     };
//
//     let client = S3Client::new(cfg).await.expect("connect");
//     client.put_object("hello.txt", bytes::Bytes::from("hello world")).await.expect("put");
//     let data = client.get_object("hello.txt").await.expect("get");
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
pub mod stream;

pub use client::{ObjectMeta, PutResult, S3Client};
pub use config::S3Config;
pub use error::S3Fault;
pub use events::{S3ObjectCreated, S3ObjectDeleted};
pub use state::S3ClientState;
