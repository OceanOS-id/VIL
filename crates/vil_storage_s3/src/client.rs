// =============================================================================
// vil_storage_s3::client — S3Client
// =============================================================================
//
// S3-compatible object storage client wrapping `aws-sdk-s3`.
//
// Every public operation:
//   1. Records `Instant::now()` before the call.
//   2. Executes the AWS SDK operation.
//   3. Emits `db_log!` with timing on both success and error paths.
//      (Storage connectors use DbLog per COMPLIANCE.md §8.)
//
// No `println!`, `tracing::info!`, or `eprintln!` are used.
// All string fields in log payloads use `register_str()` hashes.
// =============================================================================

use std::time::Instant;

use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{Builder as S3ConfigBuilder, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use bytes::Bytes;

use vil_log::dict::register_str;
use vil_log::{db_log, types::DbPayload};

use crate::config::S3Config;
use crate::error::S3Fault;
use crate::stream::collect_body;

// op_type constants (reusing DbPayload op_type field semantics)
const OP_PUT: u8 = 1;    // INSERT — put_object
const OP_GET: u8 = 0;    // SELECT — get_object
const OP_DELETE: u8 = 3; // DELETE — delete_object
const OP_LIST: u8 = 0;   // SELECT — list_objects
const OP_HEAD: u8 = 0;   // SELECT — head_object

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
    /// RFC 3339 / ISO 8601 last-modified timestamp as returned by S3, if available.
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
    inner: aws_sdk_s3::Client,
    bucket: String,
    /// FxHash of the endpoint (or "aws-s3") for log payloads.
    config_hash: u32,
}

impl S3Client {
    /// Create a new `S3Client` from the provided configuration.
    ///
    /// Credentials supplied via `S3Config::access_key` / `secret_key` take
    /// precedence over environment variables and instance metadata.
    pub async fn new(config: S3Config) -> Result<Self, S3Fault> {
        let endpoint_label = config
            .endpoint
            .as_deref()
            .unwrap_or("aws-s3");
        let config_hash = register_str(endpoint_label);

        // Build AWS SDK config ------------------------------------------------
        let region = Region::new(config.region.clone());

        let mut loader = aws_config::defaults(BehaviorVersion::latest())
            .region(region);

        // Explicit credentials override env / instance metadata
        if let (Some(ak), Some(sk)) = (config.access_key.as_deref(), config.secret_key.as_deref()) {
            let creds = Credentials::new(ak, sk, None, None, "vil_storage_s3");
            loader = loader.credentials_provider(creds);
        }

        let sdk_config = loader.load().await;

        // Build S3-specific config (custom endpoint + path style) -------------
        let mut s3_builder = S3ConfigBuilder::from(&sdk_config);

        if let Some(ref ep) = config.endpoint {
            s3_builder = s3_builder.endpoint_url(ep);
        }

        if config.path_style {
            s3_builder = s3_builder.force_path_style(true);
        }

        let s3_config = s3_builder.build();
        let inner = aws_sdk_s3::Client::from_conf(s3_config);

        Ok(Self {
            inner,
            bucket: config.bucket,
            config_hash,
        })
    }

    // =========================================================================
    // put_object
    // =========================================================================

    /// Upload `body` bytes to `key` in the configured bucket.
    ///
    /// Emits `db_log!(Info, ...)` with `op_type = OP_PUT` on success,
    /// or with `error_code = 1` on failure.
    pub async fn put_object(&self, key: &str, body: Bytes) -> Result<PutResult, S3Fault> {
        let start = Instant::now();
        let key_hash = register_str(key);
        let size = body.len() as u64;

        let result = self
            .inner
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(aws_sdk_s3::primitives::ByteStream::from(body))
            .send()
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(output) => {
                db_log!(Info, DbPayload {
                    db_hash:      self.config_hash,
                    table_hash:   register_str(&self.bucket),
                    query_hash:   key_hash,
                    duration_us:  elapsed.as_micros() as u32,
                    rows_affected: 1,
                    op_type:      OP_PUT,
                    error_code:   0,
                    ..Default::default()
                });

                Ok(PutResult {
                    e_tag: output.e_tag().map(str::to_owned),
                    version_id: output.version_id().map(str::to_owned),
                })
            }
            Err(e) => {
                let fault = classify_put_error(&e, key_hash, size);

                db_log!(Info, DbPayload {
                    db_hash:      self.config_hash,
                    table_hash:   register_str(&self.bucket),
                    query_hash:   key_hash,
                    duration_us:  elapsed.as_micros() as u32,
                    rows_affected: 0,
                    op_type:      OP_PUT,
                    error_code:   1,
                    ..Default::default()
                });

                Err(fault)
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

        let result = self
            .inner
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(output) => {
                // Collect the body stream — may yield a second error
                let body = collect_body(output.body).await;

                db_log!(Info, DbPayload {
                    db_hash:      self.config_hash,
                    table_hash:   register_str(&self.bucket),
                    query_hash:   key_hash,
                    duration_us:  elapsed.as_micros() as u32,
                    rows_affected: if body.is_ok() { 1 } else { 0 },
                    op_type:      OP_GET,
                    error_code:   if body.is_ok() { 0 } else { 1 },
                    ..Default::default()
                });

                body
            }
            Err(e) => {
                let fault = classify_get_error(&e, key_hash);

                db_log!(Info, DbPayload {
                    db_hash:      self.config_hash,
                    table_hash:   register_str(&self.bucket),
                    query_hash:   key_hash,
                    duration_us:  elapsed.as_micros() as u32,
                    rows_affected: 0,
                    op_type:      OP_GET,
                    error_code:   1,
                    ..Default::default()
                });

                Err(fault)
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

        let result = self
            .inner
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(_) => {
                db_log!(Info, DbPayload {
                    db_hash:      self.config_hash,
                    table_hash:   register_str(&self.bucket),
                    query_hash:   key_hash,
                    duration_us:  elapsed.as_micros() as u32,
                    rows_affected: 1,
                    op_type:      OP_DELETE,
                    error_code:   0,
                    ..Default::default()
                });

                Ok(())
            }
            Err(e) => {
                let fault = classify_delete_error(&e, key_hash);

                db_log!(Info, DbPayload {
                    db_hash:      self.config_hash,
                    table_hash:   register_str(&self.bucket),
                    query_hash:   key_hash,
                    duration_us:  elapsed.as_micros() as u32,
                    rows_affected: 0,
                    op_type:      OP_DELETE,
                    error_code:   1,
                    ..Default::default()
                });

                Err(fault)
            }
        }
    }

    // =========================================================================
    // list_objects
    // =========================================================================

    /// List objects whose keys share the given `prefix`.
    ///
    /// Returns up to 1000 results (AWS default page size). For paginated access
    /// over large buckets, call this method multiple times with a continuation
    /// token (not yet exposed in this API).
    pub async fn list_objects(&self, prefix: &str) -> Result<Vec<ObjectMeta>, S3Fault> {
        let start = Instant::now();
        let prefix_hash = register_str(prefix);

        let result = self
            .inner
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(output) => {
                let objects: Vec<ObjectMeta> = output
                    .contents()
                    .iter()
                    .map(|obj| ObjectMeta {
                        key: obj.key().unwrap_or_default().to_owned(),
                        size: obj.size().unwrap_or(0) as u64,
                        last_modified: obj
                            .last_modified()
                            .map(|dt| dt.fmt(aws_sdk_s3::primitives::DateTimeFormat::DateTime)
                                .unwrap_or_default()),
                        e_tag: obj.e_tag().map(str::to_owned),
                    })
                    .collect();

                let count = objects.len() as u32;

                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.bucket),
                    query_hash:    prefix_hash,
                    duration_us:   elapsed.as_micros() as u32,
                    rows_affected: count,
                    op_type:       OP_LIST,
                    error_code:    0,
                    ..Default::default()
                });

                Ok(objects)
            }
            Err(e) => {
                let fault = classify_list_error(&e, register_str(&self.bucket));

                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.bucket),
                    query_hash:    prefix_hash,
                    duration_us:   elapsed.as_micros() as u32,
                    rows_affected: 0,
                    op_type:       OP_LIST,
                    error_code:    1,
                    ..Default::default()
                });

                Err(fault)
            }
        }
    }

    // =========================================================================
    // head_object
    // =========================================================================

    /// Retrieve metadata for `key` without downloading the object body.
    ///
    /// Returns `S3Fault::NotFound` if the key does not exist.
    pub async fn head_object(&self, key: &str) -> Result<ObjectMeta, S3Fault> {
        let start = Instant::now();
        let key_hash = register_str(key);

        let result = self
            .inner
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(output) => {
                let meta = ObjectMeta {
                    key: key.to_owned(),
                    size: output.content_length().unwrap_or(0) as u64,
                    last_modified: output
                        .last_modified()
                        .map(|dt| dt.fmt(aws_sdk_s3::primitives::DateTimeFormat::DateTime)
                            .unwrap_or_default()),
                    e_tag: output.e_tag().map(str::to_owned),
                };

                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.bucket),
                    query_hash:    key_hash,
                    duration_us:   elapsed.as_micros() as u32,
                    rows_affected: 1,
                    op_type:       OP_HEAD,
                    error_code:    0,
                    ..Default::default()
                });

                Ok(meta)
            }
            Err(e) => {
                let fault = classify_head_error(&e, key_hash);

                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.bucket),
                    query_hash:    key_hash,
                    duration_us:   elapsed.as_micros() as u32,
                    rows_affected: 0,
                    op_type:       OP_HEAD,
                    error_code:    1,
                    ..Default::default()
                });

                Err(fault)
            }
        }
    }

    // =========================================================================
    // presigned_url
    // =========================================================================

    /// Generate a time-limited presigned GET URL for `key`.
    ///
    /// The URL is valid for `expires_secs` seconds and allows anonymous
    /// download of the object without AWS credentials.
    pub async fn presigned_url(&self, key: &str, expires_secs: u64) -> Result<String, S3Fault> {
        let start = Instant::now();
        let key_hash = register_str(key);

        let expires = std::time::Duration::from_secs(expires_secs);
        let presigning_config = PresigningConfig::expires_in(expires)
            .map_err(|e| S3Fault::Unknown {
                message_hash: register_str(&e.to_string()),
            })?;

        let result = self
            .inner
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(presigning_config)
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(presigned) => {
                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.bucket),
                    query_hash:    key_hash,
                    duration_us:   elapsed.as_micros() as u32,
                    rows_affected: 1,
                    op_type:       OP_GET,
                    error_code:    0,
                    ..Default::default()
                });

                Ok(presigned.uri().to_string())
            }
            Err(e) => {
                let fault = classify_presign_error(&e, key_hash);

                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.bucket),
                    query_hash:    key_hash,
                    duration_us:   elapsed.as_micros() as u32,
                    rows_affected: 0,
                    op_type:       OP_GET,
                    error_code:    1,
                    ..Default::default()
                });

                Err(fault)
            }
        }
    }
}

// =============================================================================
// Error classification helpers
// =============================================================================
//
// The AWS SDK uses a deeply-nested error hierarchy. We match on HTTP status
// codes and error codes where possible and fall back to S3Fault::Unknown.
// All string → hash conversion happens inside these functions so the hot path
// never allocates beyond what the SDK itself requires.

fn classify_put_error<E>(e: &aws_sdk_s3::error::SdkError<E>, key_hash: u32, size: u64) -> S3Fault {
    use aws_sdk_s3::error::SdkError;
    match e {
        SdkError::TimeoutError(_) => S3Fault::Timeout {
            operation_hash: register_str("put_object"),
            elapsed_ms: 0,
        },
        SdkError::DispatchFailure(_) | SdkError::ConstructionFailure(_) => {
            S3Fault::ConnectionFailed {
                endpoint_hash: register_str("put_object"),
                reason_code: 0,
            }
        }
        _ => {
            // Check HTTP status for access / bucket errors
            if let Some(status) = sdk_http_status(e) {
                match status {
                    403 | 401 => return S3Fault::AccessDenied { key_hash },
                    404 => return S3Fault::BucketNotFound {
                        bucket_hash: register_str("bucket"),
                    },
                    _ => {}
                }
            }
            S3Fault::UploadFailed { key_hash, size }
        }
    }
}

fn classify_get_error<E>(e: &aws_sdk_s3::error::SdkError<E>, key_hash: u32) -> S3Fault {
    use aws_sdk_s3::error::SdkError;
    match e {
        SdkError::TimeoutError(_) => S3Fault::Timeout {
            operation_hash: register_str("get_object"),
            elapsed_ms: 0,
        },
        SdkError::DispatchFailure(_) | SdkError::ConstructionFailure(_) => {
            S3Fault::ConnectionFailed {
                endpoint_hash: register_str("get_object"),
                reason_code: 0,
            }
        }
        _ => {
            if let Some(status) = sdk_http_status(e) {
                match status {
                    403 | 401 => return S3Fault::AccessDenied { key_hash },
                    404 => return S3Fault::NotFound { key_hash },
                    _ => {}
                }
            }
            S3Fault::Unknown {
                message_hash: register_str("get_object_error"),
            }
        }
    }
}

fn classify_delete_error<E>(e: &aws_sdk_s3::error::SdkError<E>, key_hash: u32) -> S3Fault {
    use aws_sdk_s3::error::SdkError;
    match e {
        SdkError::TimeoutError(_) => S3Fault::Timeout {
            operation_hash: register_str("delete_object"),
            elapsed_ms: 0,
        },
        SdkError::DispatchFailure(_) | SdkError::ConstructionFailure(_) => {
            S3Fault::ConnectionFailed {
                endpoint_hash: register_str("delete_object"),
                reason_code: 0,
            }
        }
        _ => {
            if let Some(status) = sdk_http_status(e) {
                match status {
                    403 | 401 => return S3Fault::AccessDenied { key_hash },
                    404 => return S3Fault::BucketNotFound {
                        bucket_hash: register_str("bucket"),
                    },
                    _ => {}
                }
            }
            S3Fault::Unknown {
                message_hash: register_str("delete_object_error"),
            }
        }
    }
}

fn classify_list_error<E>(e: &aws_sdk_s3::error::SdkError<E>, bucket_hash: u32) -> S3Fault {
    use aws_sdk_s3::error::SdkError;
    match e {
        SdkError::TimeoutError(_) => S3Fault::Timeout {
            operation_hash: register_str("list_objects"),
            elapsed_ms: 0,
        },
        SdkError::DispatchFailure(_) | SdkError::ConstructionFailure(_) => {
            S3Fault::ConnectionFailed {
                endpoint_hash: register_str("list_objects"),
                reason_code: 0,
            }
        }
        _ => {
            if let Some(status) = sdk_http_status(e) {
                match status {
                    403 | 401 => return S3Fault::AccessDenied {
                        key_hash: bucket_hash,
                    },
                    404 => return S3Fault::BucketNotFound { bucket_hash },
                    _ => {}
                }
            }
            S3Fault::Unknown {
                message_hash: register_str("list_objects_error"),
            }
        }
    }
}

fn classify_head_error<E>(e: &aws_sdk_s3::error::SdkError<E>, key_hash: u32) -> S3Fault {
    use aws_sdk_s3::error::SdkError;
    match e {
        SdkError::TimeoutError(_) => S3Fault::Timeout {
            operation_hash: register_str("head_object"),
            elapsed_ms: 0,
        },
        SdkError::DispatchFailure(_) | SdkError::ConstructionFailure(_) => {
            S3Fault::ConnectionFailed {
                endpoint_hash: register_str("head_object"),
                reason_code: 0,
            }
        }
        _ => {
            if let Some(status) = sdk_http_status(e) {
                match status {
                    403 | 401 => return S3Fault::AccessDenied { key_hash },
                    404 => return S3Fault::NotFound { key_hash },
                    _ => {}
                }
            }
            S3Fault::Unknown {
                message_hash: register_str("head_object_error"),
            }
        }
    }
}

fn classify_presign_error<E>(e: &aws_sdk_s3::error::SdkError<E>, key_hash: u32) -> S3Fault {
    use aws_sdk_s3::error::SdkError;
    match e {
        SdkError::TimeoutError(_) => S3Fault::Timeout {
            operation_hash: register_str("presigned_url"),
            elapsed_ms: 0,
        },
        _ => {
            if let Some(status) = sdk_http_status(e) {
                match status {
                    403 | 401 => return S3Fault::AccessDenied { key_hash },
                    404 => return S3Fault::NotFound { key_hash },
                    _ => {}
                }
            }
            S3Fault::Unknown {
                message_hash: register_str("presign_error"),
            }
        }
    }
}

/// Extract the HTTP status code from an `SdkError` if available.
fn sdk_http_status<E>(e: &aws_sdk_s3::error::SdkError<E>) -> Option<u16> {
    use aws_sdk_s3::error::SdkError;
    if let SdkError::ServiceError(svc) = e {
        Some(svc.raw().status().as_u16())
    } else {
        None
    }
}
