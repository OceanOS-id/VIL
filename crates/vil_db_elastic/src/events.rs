// =============================================================================
// vil_db_elastic::events — Elasticsearch connector events
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when a document is successfully indexed into Elasticsearch.
#[connector_event]
pub struct ElasticDocumentIndexed {
    pub index_hash: u32,
    pub id_hash: u32,
    pub elapsed_ns: u64,
    pub timestamp_ns: u64,
}

/// Emitted when a search query is successfully executed.
#[connector_event]
pub struct ElasticSearchExecuted {
    pub index_hash: u32,
    pub query_hash: u32,
    pub hits: u32,
    pub elapsed_ns: u64,
    pub timestamp_ns: u64,
}
