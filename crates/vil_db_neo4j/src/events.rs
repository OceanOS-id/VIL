// =============================================================================
// vil_db_neo4j::events — Neo4j connector events
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when a node is successfully created in Neo4j.
#[connector_event]
pub struct Neo4jNodeCreated {
    pub label_hash: u32,
    pub node_id_hash: u32,
    pub timestamp_ns: u64,
}

/// Emitted when a relationship is successfully created in Neo4j.
#[connector_event]
pub struct Neo4jRelationCreated {
    pub rel_type_hash: u32,
    pub from_id_hash: u32,
    pub to_id_hash: u32,
    pub timestamp_ns: u64,
}
