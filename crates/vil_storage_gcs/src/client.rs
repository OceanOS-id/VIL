// =============================================================================
// vil_storage_gcs::client — GcsClient
// =============================================================================
//
// GCS object storage client wrapping `google-cloud-storage`.
//
// Every public operation:
//   1. Records `Instant::now()` before the call.
//   2. Executes the GCS SDK operation.
//   3. Emits `db_log!` with timing on both success and error paths.
//
// No `println!`, `tracing::info!`, or `eprintln!` are used.
// All string fields in log payloads use `register_str()` hashes.
// =============================================================================

use std::time::Instant;

use bytes::Bytes;
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::delete::DeleteObjectRequest;
use google_cloud_storage::http::objects::download::Range;
use google_cloud_storage::http::objects::get::GetObjectRequest;
use google_cloud_storage::http::objects::list::ListObjectsRequest;
use google_cloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};

use vil_log::dict::register_str;
use vil_log::{db_log, types::DbPayload};

use crate::config::GcsConfig;
use crate::error::GcsFault;

// op_type constants
const OP_GET: u8 = 0;    // SELECT — download
const OP_PUT: u8 = 1;    // INSERT — upload
const OP_DELETE: u8 = 3; // DELETE — delete
const OP_LIST: u8 = 0;   // SELECT — list
const OP_HEAD: u8 = 0;   // SELECT — get_metadata

// =============================================================================
// Result types
// =============================================================================

/// Result returned by a successful `upload` call.
#[derive(Debug, Clone)]
pub struct GcsUploadResult {
    /// The generation number of the uploaded object.
    pub generation: Option<i64>,
    /// The ETag of the uploaded object, if returned.
    pub e_tag: Option<String>,
}

/// Metadata for a stored GCS object.
#[derive(Debug, Clone)]
pub struct GcsBlobMeta {
    /// The full object name (key).
    pub name: String,
    /// Size of the object in bytes.
    pub size: u64,
    /// RFC 3339 last-modified timestamp, if available.
    pub updated: Option<String>,
    /// Content type, if available.
    pub content_type: Option<String>,
}

// =============================================================================
// GcsClient
// =============================================================================

/// Google Cloud Storage client.
///
/// Build one via [`GcsClient::new`] with a [`GcsConfig`], then use the async
/// methods to interact with the configured bucket.
///
/// Every method auto-emits a `db_log!` entry with operation timing.
pub struct GcsClient {
    inner: Client,
    bucket: String,
    /// FxHash of the bucket name for log payloads.
    config_hash: u32,
}

impl GcsClient {
    /// Create a new `GcsClient` from the provided configuration.
    pub async fn new(config: GcsConfig) -> Result<Self, GcsFault> {
        let config_hash = register_str(&config.bucket);

        let gcs_config = ClientConfig::default()
            .with_auth()
            .await
            .map_err(|e| GcsFault::ConnectionFailed {
                endpoint_hash: register_str(&e.to_string()),
                reason_code: 0,
            })?;

        let inner = Client::new(gcs_config);

        Ok(Self {
            inner,
            bucket: config.bucket,
            config_hash,
        })
    }

    // =========================================================================
    // upload
    // =========================================================================

    /// Upload `body` bytes to `name` in the configured bucket.
    pub async fn upload(&self, name: &str, body: Bytes) -> Result<GcsUploadResult, GcsFault> {
        let start = Instant::now();
        let name_hash = register_str(name);
        let size = body.len() as u64;

        let upload_type = UploadType::Simple(Media::new(name.to_owned()));
        let result = self
            .inner
            .upload_object(
                &UploadObjectRequest {
                    bucket: self.bucket.clone(),
                    ..Default::default()
                },
                body,
                &upload_type,
            )
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(obj) => {
                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.bucket),
                    query_hash:    name_hash,
                    duration_us:   elapsed.as_micros() as u32,
                    rows_affected: 1,
                    op_type:       OP_PUT,
                    error_code:    0,
                    ..Default::default()
                });

                Ok(GcsUploadResult {
                    generation: Some(obj.generation),
                    e_tag: Some(obj.etag),
                })
            }
            Err(e) => {
                let fault = classify_error(&e, name_hash, Some(size));

                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.bucket),
                    query_hash:    name_hash,
                    duration_us:   elapsed.as_micros() as u32,
                    rows_affected: 0,
                    op_type:       OP_PUT,
                    error_code:    1,
                    ..Default::default()
                });

                Err(fault)
            }
        }
    }

    // =========================================================================
    // download
    // =========================================================================

    /// Download the object at `name` and return its contents as `Bytes`.
    pub async fn download(&self, name: &str) -> Result<Bytes, GcsFault> {
        let start = Instant::now();
        let name_hash = register_str(name);

        let result = self
            .inner
            .download_object(
                &GetObjectRequest {
                    bucket: self.bucket.clone(),
                    object: name.to_owned(),
                    ..Default::default()
                },
                &Range::default(),
            )
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(data) => {
                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.bucket),
                    query_hash:    name_hash,
                    duration_us:   elapsed.as_micros() as u32,
                    rows_affected: 1,
                    op_type:       OP_GET,
                    error_code:    0,
                    ..Default::default()
                });

                Ok(Bytes::from(data))
            }
            Err(e) => {
                let fault = classify_error(&e, name_hash, None);

                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.bucket),
                    query_hash:    name_hash,
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

    // =========================================================================
    // delete
    // =========================================================================

    /// Delete the object at `name` from the configured bucket.
    pub async fn delete(&self, name: &str) -> Result<(), GcsFault> {
        let start = Instant::now();
        let name_hash = register_str(name);

        let result = self
            .inner
            .delete_object(&DeleteObjectRequest {
                bucket: self.bucket.clone(),
                object: name.to_owned(),
                ..Default::default()
            })
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(_) => {
                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.bucket),
                    query_hash:    name_hash,
                    duration_us:   elapsed.as_micros() as u32,
                    rows_affected: 1,
                    op_type:       OP_DELETE,
                    error_code:    0,
                    ..Default::default()
                });

                Ok(())
            }
            Err(e) => {
                let fault = classify_error(&e, name_hash, None);

                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.bucket),
                    query_hash:    name_hash,
                    duration_us:   elapsed.as_micros() as u32,
                    rows_affected: 0,
                    op_type:       OP_DELETE,
                    error_code:    1,
                    ..Default::default()
                });

                Err(fault)
            }
        }
    }

    // =========================================================================
    // list
    // =========================================================================

    /// List objects whose names share the given `prefix`.
    pub async fn list(&self, prefix: &str) -> Result<Vec<GcsBlobMeta>, GcsFault> {
        let start = Instant::now();
        let prefix_hash = register_str(prefix);

        let result = self
            .inner
            .list_objects(&ListObjectsRequest {
                bucket: self.bucket.clone(),
                prefix: Some(prefix.to_owned()),
                ..Default::default()
            })
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(response) => {
                let objects: Vec<GcsBlobMeta> = response
                    .items
                    .unwrap_or_default()
                    .into_iter()
                    .map(|obj| GcsBlobMeta {
                        name: obj.name,
                        size: obj.size as u64,
                        updated: obj.updated.map(|t| t.to_string()),
                        content_type: obj.content_type,
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
                let fault = classify_error(&e, register_str(&self.bucket), None);

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
    // get_metadata
    // =========================================================================

    /// Retrieve metadata for `name` without downloading the object body.
    pub async fn get_metadata(&self, name: &str) -> Result<GcsBlobMeta, GcsFault> {
        let start = Instant::now();
        let name_hash = register_str(name);

        let result = self
            .inner
            .get_object(&GetObjectRequest {
                bucket: self.bucket.clone(),
                object: name.to_owned(),
                ..Default::default()
            })
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(obj) => {
                let meta = GcsBlobMeta {
                    name: obj.name,
                    size: obj.size as u64,
                    updated: obj.updated.map(|t| t.to_string()),
                    content_type: obj.content_type,
                };

                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.bucket),
                    query_hash:    name_hash,
                    duration_us:   elapsed.as_micros() as u32,
                    rows_affected: 1,
                    op_type:       OP_HEAD,
                    error_code:    0,
                    ..Default::default()
                });

                Ok(meta)
            }
            Err(e) => {
                let fault = classify_error(&e, name_hash, None);

                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.bucket),
                    query_hash:    name_hash,
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
}

// =============================================================================
// Error classification helpers
// =============================================================================

fn classify_error(
    e: &google_cloud_storage::http::Error,
    name_hash: u32,
    size: Option<u64>,
) -> GcsFault {
    use google_cloud_storage::http::Error;
    match e {
        Error::HttpClient(_) => GcsFault::ConnectionFailed {
            endpoint_hash: register_str("gcs"),
            reason_code: 0,
        },
        Error::Response(status) => {
            match status.code {
                401 | 403 => GcsFault::AccessDenied { name_hash },
                404 => GcsFault::NotFound { name_hash },
                _ => {
                    if let Some(s) = size {
                        GcsFault::UploadFailed { name_hash, size: s }
                    } else {
                        GcsFault::Unknown {
                            message_hash: register_str("gcs_response_error"),
                        }
                    }
                }
            }
        }
        Error::TokenSource(_) => GcsFault::AccessDenied { name_hash },
        _ => GcsFault::Unknown {
            message_hash: register_str("gcs_error"),
        },
    }
}
