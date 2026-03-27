// =============================================================================
// vil_storage_s3::events — S3 connector events
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when an object is successfully created/uploaded to S3.
#[connector_event]
pub struct S3ObjectCreated {
    pub bucket_hash: u32,
    pub key_hash: u32,
    pub size_bytes: u64,
    pub etag_hash: u32,
    pub timestamp_ns: u64,
}

/// Emitted when an object is successfully deleted from S3.
#[connector_event]
pub struct S3ObjectDeleted {
    pub bucket_hash: u32,
    pub key_hash: u32,
    pub timestamp_ns: u64,
}
