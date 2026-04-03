// =============================================================================
// vil_db_clickhouse::events — ClickHouse connector events
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when a batch INSERT is successfully committed to ClickHouse.
#[connector_event]
pub struct ChBatchInserted {
    pub table_hash: u32,
    pub rows: u64,
    pub elapsed_ns: u32,
    pub timestamp_ns: u64,
}
