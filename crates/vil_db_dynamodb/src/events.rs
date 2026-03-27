// =============================================================================
// vil_db_dynamodb::events — DynamoDB connector events
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when an item is successfully put into a DynamoDB table.
#[connector_event]
pub struct DynamoItemPut {
    pub table_hash: u32,
    pub key_hash: u32,
    pub timestamp_ns: u64,
}

/// Emitted when an item is successfully deleted from a DynamoDB table.
#[connector_event]
pub struct DynamoItemDeleted {
    pub table_hash: u32,
    pub key_hash: u32,
    pub timestamp_ns: u64,
}
