// =============================================================================
// vil_storage_azure::client — AzureClient
// =============================================================================
//
// Azure Blob Storage client wrapping `azure_storage_blobs`.
//
// Every public operation:
//   1. Records `Instant::now()` before the call.
//   2. Executes the Azure SDK operation.
//   3. Emits `db_log!` with timing on both success and error paths.
//
// No `println!`, `tracing::info!`, or `eprintln!` are used.
// All string fields in log payloads use `register_str()` hashes.
// =============================================================================

use std::time::Instant;

use azure_storage::StorageCredentials;
use azure_storage_blobs::prelude::{BlobServiceClient, ContainerClient};
use bytes::Bytes;

use vil_log::dict::register_str;
use vil_log::{db_log, types::DbPayload};

use crate::config::AzureConfig;
use crate::error::AzureFault;

// op_type constants
const OP_GET: u8 = 0;    // SELECT — download_blob
const OP_PUT: u8 = 1;    // INSERT — upload_blob
const OP_DELETE: u8 = 3; // DELETE — delete_blob
const OP_LIST: u8 = 0;   // SELECT — list_blobs
const OP_HEAD: u8 = 0;   // SELECT — get_properties

// =============================================================================
// Result types
// =============================================================================

/// Result returned by a successful `upload_blob` call.
#[derive(Debug, Clone)]
pub struct AzureUploadResult {
    /// The ETag of the uploaded blob, if returned.
    pub e_tag: Option<String>,
    /// The last-modified timestamp string, if returned.
    pub last_modified: Option<String>,
}

/// Metadata for a stored Azure blob.
#[derive(Debug, Clone)]
pub struct AzureBlobMeta {
    /// The blob name.
    pub name: String,
    /// Size of the blob in bytes.
    pub size: u64,
    /// RFC 1123 last-modified timestamp, if available.
    pub last_modified: Option<String>,
    /// Content type, if available.
    pub content_type: Option<String>,
    /// The ETag of the blob, if available.
    pub e_tag: Option<String>,
}

// =============================================================================
// AzureClient
// =============================================================================

/// Azure Blob Storage client.
///
/// Build one via [`AzureClient::new`] with an [`AzureConfig`], then use the
/// async methods to interact with the configured container.
///
/// Every method auto-emits a `db_log!` entry with operation timing.
pub struct AzureClient {
    container_client: ContainerClient,
    container: String,
    /// FxHash of the account name for log payloads.
    config_hash: u32,
}

impl AzureClient {
    /// Create a new `AzureClient` from the provided configuration.
    pub fn new(config: AzureConfig) -> Result<Self, AzureFault> {
        let config_hash = register_str(&config.account);

        let credentials = StorageCredentials::access_key(
            config.account.clone(),
            config.access_key.clone(),
        );

        let service_client = BlobServiceClient::new(config.account.clone(), credentials);
        let container_client = service_client.container_client(config.container.clone());

        Ok(Self {
            container_client,
            container: config.container,
            config_hash,
        })
    }

    // =========================================================================
    // upload_blob
    // =========================================================================

    /// Upload `body` bytes as a block blob named `name`.
    pub async fn upload_blob(&self, name: &str, body: Bytes) -> Result<AzureUploadResult, AzureFault> {
        let start = Instant::now();
        let name_hash = register_str(name);
        let size = body.len() as u64;

        let blob_client = self.container_client.blob_client(name);
        let result = blob_client
            .put_block_blob(body)
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(response) => {
                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.container),
                    query_hash:    name_hash,
                    duration_us:   elapsed.as_micros() as u32,
                    rows_affected: 1,
                    op_type:       OP_PUT,
                    error_code:    0,
                    ..Default::default()
                });

                Ok(AzureUploadResult {
                    e_tag: Some(response.etag.to_string()),
                    last_modified: Some(response.last_modified.to_string()),
                })
            }
            Err(e) => {
                let fault = classify_azure_error(&e, name_hash, Some(size), &self.container);

                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.container),
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
    // download_blob
    // =========================================================================

    /// Download the blob named `name` and return its contents as `Bytes`.
    pub async fn download_blob(&self, name: &str) -> Result<Bytes, AzureFault> {
        let start = Instant::now();
        let name_hash = register_str(name);

        let blob_client = self.container_client.blob_client(name);
        let result = blob_client.get_content().await;

        let elapsed = start.elapsed();

        match result {
            Ok(data) => {
                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.container),
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
                let fault = classify_azure_error(&e, name_hash, None, &self.container);

                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.container),
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
    // delete_blob
    // =========================================================================

    /// Delete the blob named `name` from the configured container.
    pub async fn delete_blob(&self, name: &str) -> Result<(), AzureFault> {
        let start = Instant::now();
        let name_hash = register_str(name);

        let blob_client = self.container_client.blob_client(name);
        let result = blob_client.delete().await;

        let elapsed = start.elapsed();

        match result {
            Ok(_) => {
                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.container),
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
                let fault = classify_azure_error(&e, name_hash, None, &self.container);

                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.container),
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
    // list_blobs
    // =========================================================================

    /// List blobs whose names share the given `prefix`.
    pub async fn list_blobs(&self, prefix: &str) -> Result<Vec<AzureBlobMeta>, AzureFault> {
        use azure_storage_blobs::container::operations::BlobItem;
        use futures_util::StreamExt;

        let start = Instant::now();
        let prefix_hash = register_str(prefix);

        let mut stream = self
            .container_client
            .list_blobs()
            .prefix(prefix.to_owned())
            .into_stream();

        let mut blobs = Vec::new();
        let mut error: Option<AzureFault> = None;

        while let Some(page) = stream.next().await {
            match page {
                Ok(response) => {
                    for item in response.blobs.items {
                        if let BlobItem::Blob(blob) = item {
                            blobs.push(AzureBlobMeta {
                                name: blob.name,
                                size: blob.properties.content_length,
                                last_modified: Some(blob.properties.last_modified.to_string()),
                                content_type: Some(blob.properties.content_type),
                                e_tag: Some(blob.properties.etag.to_string()),
                            });
                        }
                    }
                }
                Err(e) => {
                    error = Some(classify_azure_error(
                        &e,
                        register_str(&self.container),
                        None,
                        &self.container,
                    ));
                    break;
                }
            }
        }

        let elapsed = start.elapsed();

        if let Some(fault) = error {
            db_log!(Info, DbPayload {
                db_hash:       self.config_hash,
                table_hash:    register_str(&self.container),
                query_hash:    prefix_hash,
                duration_us:   elapsed.as_micros() as u32,
                rows_affected: 0,
                op_type:       OP_LIST,
                error_code:    1,
                ..Default::default()
            });

            return Err(fault);
        }

        let count = blobs.len() as u32;

        db_log!(Info, DbPayload {
            db_hash:       self.config_hash,
            table_hash:    register_str(&self.container),
            query_hash:    prefix_hash,
            duration_us:   elapsed.as_micros() as u32,
            rows_affected: count,
            op_type:       OP_LIST,
            error_code:    0,
            ..Default::default()
        });

        Ok(blobs)
    }

    // =========================================================================
    // get_properties
    // =========================================================================

    /// Retrieve properties for the blob named `name` without downloading the body.
    pub async fn get_properties(&self, name: &str) -> Result<AzureBlobMeta, AzureFault> {
        let start = Instant::now();
        let name_hash = register_str(name);

        let blob_client = self.container_client.blob_client(name);
        let result = blob_client.get_properties().await;

        let elapsed = start.elapsed();

        match result {
            Ok(response) => {
                let meta = AzureBlobMeta {
                    name: name.to_owned(),
                    size: response.blob.properties.content_length,
                    last_modified: Some(response.blob.properties.last_modified.to_string()),
                    content_type: Some(response.blob.properties.content_type),
                    e_tag: Some(response.blob.properties.etag.to_string()),
                };

                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.container),
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
                let fault = classify_azure_error(&e, name_hash, None, &self.container);

                db_log!(Info, DbPayload {
                    db_hash:       self.config_hash,
                    table_hash:    register_str(&self.container),
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

fn classify_azure_error(
    e: &azure_storage::Error,
    name_hash: u32,
    size: Option<u64>,
    container: &str,
) -> AzureFault {
    let msg = e.to_string();
    let msg_lower = msg.to_lowercase();

    if msg_lower.contains("401") || msg_lower.contains("403") || msg_lower.contains("unauthorized") || msg_lower.contains("forbidden") {
        return AzureFault::AccessDenied { name_hash };
    }

    if msg_lower.contains("404") || msg_lower.contains("not found") || msg_lower.contains("blobnotfound") {
        if msg_lower.contains("container") || msg_lower.contains("containernotfound") {
            return AzureFault::ContainerNotFound {
                container_hash: register_str(container),
            };
        }
        return AzureFault::NotFound { name_hash };
    }

    if msg_lower.contains("timeout") || msg_lower.contains("timed out") {
        return AzureFault::Timeout {
            operation_hash: register_str("azure_operation"),
            elapsed_ms: 0,
        };
    }

    if msg_lower.contains("connection") || msg_lower.contains("connect") {
        return AzureFault::ConnectionFailed {
            account_hash: register_str("azure"),
            reason_code: 0,
        };
    }

    if let Some(s) = size {
        return AzureFault::UploadFailed { name_hash, size: s };
    }

    AzureFault::Unknown {
        message_hash: register_str(&msg),
    }
}
