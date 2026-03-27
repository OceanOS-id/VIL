// =============================================================================
// vil_storage_azure::events — Azure Blob Storage connector events
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when a blob is successfully created/uploaded to Azure.
#[connector_event]
pub struct AzureBlobCreated {
    pub container_hash: u32,
    pub name_hash: u32,
    pub size_bytes: u64,
    pub etag_hash: u32,
    pub timestamp_ns: u64,
}

/// Emitted when a blob is successfully deleted from Azure.
#[connector_event]
pub struct AzureBlobDeleted {
    pub container_hash: u32,
    pub name_hash: u32,
    pub timestamp_ns: u64,
}
