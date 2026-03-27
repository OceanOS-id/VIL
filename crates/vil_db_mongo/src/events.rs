// =============================================================================
// vil_db_mongo::events — MongoDB connector events
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when a document is successfully inserted into a collection.
#[connector_event]
pub struct MongoDocumentInserted {
    pub collection_hash: u32,
    pub document_id_hash: u32,
    pub timestamp_ns: u64,
}

/// Emitted when a document is successfully updated in a collection.
#[connector_event]
pub struct MongoDocumentUpdated {
    pub collection_hash: u32,
    pub document_id_hash: u32,
    pub timestamp_ns: u64,
}

/// Emitted when a document is successfully deleted from a collection.
#[connector_event]
pub struct MongoDocumentDeleted {
    pub collection_hash: u32,
    pub document_id_hash: u32,
    pub timestamp_ns: u64,
}
