// =============================================================================
// vil_storage_gcs::events — GCS connector events
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when an object is successfully created/uploaded to GCS.
#[connector_event]
pub struct GcsObjectCreated {
    pub bucket_hash: u32,
    pub name_hash: u32,
    pub size_bytes: u64,
    pub etag_hash: u32,
    pub timestamp_ns: u64,
}

/// Emitted when an object is successfully deleted from GCS.
#[connector_event]
pub struct GcsObjectDeleted {
    pub bucket_hash: u32,
    pub name_hash: u32,
    pub timestamp_ns: u64,
}
