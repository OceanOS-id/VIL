// =============================================================================
// vil_storage_s3::client — S3Client (MinIO SDK)
// =============================================================================
//
// S3-compatible object storage client wrapping the MinIO Rust SDK.
// Compatible with AWS S3, MinIO, Cloudflare R2, DigitalOcean Spaces,
// and any service implementing the S3 REST API.
//
// Every public operation emits `db_log!` with timing per COMPLIANCE.md §8.
// No `println!`, `tracing::info!`, or `eprintln!` in production code.
// All string fields in log payloads use `register_str()` hashes.
// =============================================================================

use std::time::Instant;

use bytes::Bytes;
use minio::s3::creds::StaticProvider;
use minio::s3::http::BaseUrl;
use minio::s3::types::{S3Api, ToStream};
use minio::s3::Client;
use tokio_stream::StreamExt;

use vil_log::dict::register_str;
use vil_log::{db_log, types::DbPayload};

use crate::config::S3Config;
use crate::error::S3Fault;
use crate::stream::collect_content;

// op_type constants (reusing DbPayload op_type field semantics)
const OP_PUT: u8 = 1; // INSERT — put_object
const OP_GET: u8 = 0; // SELECT — get_object
const OP_DELETE: u8 = 3; // DELETE — delete_object
const OP_LIST: u8 = 0; // SELECT — list_objects
const OP_HEAD: u8 = 0; // SELECT — stat_object

// =============================================================================
// Result types
// =============================================================================

/// Result returned by a successful `put_object` call.
#[derive(Debug, Clone)]
pub struct PutResult {
    /// The ETag of the uploaded object, if returned by the server.
    pub e_tag: Option<String>,
    /// The version ID of the uploaded object (only when bucket versioning is enabled).
    pub version_id: Option<String>,
}

/// Metadata for a stored object.
#[derive(Debug, Clone)]
pub struct ObjectMeta {
    /// The full object key.
    pub key: String,
    /// Size of the object in bytes.
    pub size: u64,
    /// Last-modified timestamp as string, if available.
    pub last_modified: Option<String>,
    /// The ETag of the object, if available.
    pub e_tag: Option<String>,
}

// =============================================================================
// S3Client
// =============================================================================

/// S3-compatible object storage client.
///
/// Build one via [`S3Client::new`] with an [`S3Config`], then use the async
/// methods to interact with the configured bucket.
///
/// Every method auto-emits a `db_log!` entry with operation timing so that
/// VIL's semantic log drain can record storage latencies without any
/// additional instrumentation in the caller.
///
/// This crate spawns **no** internal threads. Add 0 to your
/// `LogConfig.threads` budget.
pub struct S3Client {
    inner: Client,
    bucket: String,
    /// Hash of the endpoint for log payloads.
    config_hash: u32,
}

impl S3Client {
    /// Create a new `S3Client` from the provided configuration.
    ///
    /// Credentials supplied via `S3Config::access_key` / `secret_key` take
    /// precedence. If not provided, anonymous access is used.
    pub async fn new(config: S3Config) -> Result<Self, S3Fault> {
        let endpoint = config
            .endpoint
            .as_deref()
            .unwrap_or("https://s3.amazonaws.com");
        let config_hash = register_str(endpoint);

        let base_url: BaseUrl = endpoint.parse().map_err(|_| S3Fault::ConnectionFailed {
            endpoint_hash: config_hash,
            reason_code: 1,
        })?;

        let provider: Option<Box<dyn minio::s3::creds::Provider + Send + Sync>> =
            match (config.access_key.as_deref(), config.secret_key.as_deref()) {
                (Some(ak), Some(sk)) => Some(Box::new(StaticProvider::new(ak, sk, None))),
                _ => None,
            };

        let inner =
            Client::new(base_url, provider, None, None).map_err(|_| S3Fault::ConnectionFailed {
                endpoint_hash: config_hash,
                reason_code: 2,
            })?;

        Ok(Self {
            inner,
            bucket: config.bucket,
            config_hash,
        })
    }

    /// Emit a db_log! entry for an S3 operation.
    fn emit_log(&self, key_hash: u32, elapsed_ns: u64, rows: u32, op: u8, err: u8) {
        db_log!(
            Info,
            DbPayload {
                db_hash: self.config_hash,
                table_hash: register_str(&self.bucket),
                query_hash: key_hash,
                duration_ns: elapsed_ns,
                rows_affected: rows,
                op_type: op,
                error_code: err,
                ..Default::default()
            }
        );
    }

    // =========================================================================
    // put_object
    // =========================================================================

    /// Upload `body` bytes to `key` in the configured bucket.
    pub async fn put_object(&self, key: &str, body: Bytes) -> Result<PutResult, S3Fault> {
        let start = Instant::now();
        let key_hash = register_str(key);
        let size = body.len() as u64;

        let result = self
            .inner
            .put_object_content(&self.bucket, key, body)
            .send()
            .await;

        let ns = start.elapsed().as_nanos() as u64;

        match result {
            Ok(resp) => {
                self.emit_log(key_hash, ns, 1, OP_PUT, 0);
                Ok(PutResult {
                    e_tag: Some(resp.etag),
                    version_id: resp.version_id,
                })
            }
            Err(_) => {
                self.emit_log(key_hash, ns, 0, OP_PUT, 1);
                Err(S3Fault::UploadFailed { key_hash, size })
            }
        }
    }

    // =========================================================================
    // get_object
    // =========================================================================

    /// Download the object at `key` and return its contents as `Bytes`.
    ///
    /// Returns `S3Fault::NotFound` if the key does not exist.
    pub async fn get_object(&self, key: &str) -> Result<Bytes, S3Fault> {
        let start = Instant::now();
        let key_hash = register_str(key);

        let result = self.inner.get_object(&self.bucket, key).send().await;

        let ns = start.elapsed().as_nanos() as u64;

        match result {
            Ok(resp) => match collect_content(resp.content).await {
                Ok(data) => {
                    self.emit_log(key_hash, ns, 1, OP_GET, 0);
                    Ok(data)
                }
                Err(fault) => {
                    self.emit_log(key_hash, ns, 0, OP_GET, 1);
                    Err(fault)
                }
            },
            Err(_) => {
                self.emit_log(key_hash, ns, 0, OP_GET, 1);
                Err(S3Fault::NotFound { key_hash })
            }
        }
    }

    // =========================================================================
    // delete_object
    // =========================================================================

    /// Delete the object at `key` from the configured bucket.
    ///
    /// S3 delete is idempotent; deleting a non-existent key is not an error.
    pub async fn delete_object(&self, key: &str) -> Result<(), S3Fault> {
        let start = Instant::now();
        let key_hash = register_str(key);

        let result = self.inner.delete_object(&self.bucket, key).send().await;

        let ns = start.elapsed().as_nanos() as u64;

        match result {
            Ok(_) => {
                self.emit_log(key_hash, ns, 1, OP_DELETE, 0);
                Ok(())
            }
            Err(_) => {
                self.emit_log(key_hash, ns, 0, OP_DELETE, 1);
                Err(S3Fault::Unknown {
                    message_hash: register_str("delete_object_error"),
                })
            }
        }
    }

    // =========================================================================
    // list_objects
    // =========================================================================

    /// List objects whose keys share the given `prefix`.
    ///
    /// Streams all matching entries from S3 (handles pagination internally).
    pub async fn list_objects(&self, prefix: &str) -> Result<Vec<ObjectMeta>, S3Fault> {
        let start = Instant::now();
        let prefix_hash = register_str(prefix);

        let mut stream = self
            .inner
            .list_objects(&self.bucket)
            .prefix(Some(prefix.to_string()))
            .to_stream()
            .await;

        let mut objects = Vec::new();
        while let Some(page) = stream.next().await {
            match page {
                Ok(resp) => {
                    for entry in resp.contents {
                        if entry.is_prefix {
                            continue; // skip directory markers
                        }
                        objects.push(ObjectMeta {
                            key: entry.name,
                            size: entry.size.unwrap_or(0),
                            last_modified: entry
                                .last_modified
                                .as_ref()
                                .map(|t| t.to_string()),
                            e_tag: entry.etag,
                        });
                    }
                }
                Err(_) => {
                    let ns = start.elapsed().as_nanos() as u64;
                    self.emit_log(prefix_hash, ns, 0, OP_LIST, 1);
                    return Err(S3Fault::BucketNotFound {
                        bucket_hash: register_str(&self.bucket),
                    });
                }
            }
        }

        let count = objects.len() as u32;
        let ns = start.elapsed().as_nanos() as u64;
        self.emit_log(prefix_hash, ns, count, OP_LIST, 0);

        Ok(objects)
    }

    // =========================================================================
    // head_object (stat_object)
    // =========================================================================

    /// Retrieve metadata for `key` without downloading the object body.
    ///
    /// Returns `S3Fault::NotFound` if the key does not exist.
    pub async fn head_object(&self, key: &str) -> Result<ObjectMeta, S3Fault> {
        let start = Instant::now();
        let key_hash = register_str(key);

        let result = self.inner.stat_object(&self.bucket, key).send().await;

        let ns = start.elapsed().as_nanos() as u64;

        match result {
            Ok(resp) => {
                self.emit_log(key_hash, ns, 1, OP_HEAD, 0);
                Ok(ObjectMeta {
                    key: key.to_owned(),
                    size: resp.size,
                    last_modified: resp.last_modified.as_ref().map(|t| t.to_string()),
                    e_tag: Some(resp.etag),
                })
            }
            Err(_) => {
                self.emit_log(key_hash, ns, 0, OP_HEAD, 1);
                Err(S3Fault::NotFound { key_hash })
            }
        }
    }

    // =========================================================================
    // presigned_url
    // =========================================================================

    /// Generate a time-limited presigned GET URL for `key`.
    ///
    /// The URL is valid for `expires_secs` seconds and allows anonymous
    /// download of the object without credentials.
    pub async fn presigned_url(&self, key: &str, expires_secs: u64) -> Result<String, S3Fault> {
        let start = Instant::now();
        let key_hash = register_str(key);

        let result = self
            .inner
            .get_presigned_object_url(&self.bucket, key, http::Method::GET)
            .expiry_seconds(expires_secs as u32)
            .send()
            .await;

        let ns = start.elapsed().as_nanos() as u64;

        match result {
            Ok(resp) => {
                self.emit_log(key_hash, ns, 1, OP_GET, 0);
                Ok(resp.url)
            }
            Err(_) => {
                self.emit_log(key_hash, ns, 0, OP_GET, 1);
                Err(S3Fault::Unknown {
                    message_hash: register_str("presign_error"),
                })
            }
        }
    }
}
