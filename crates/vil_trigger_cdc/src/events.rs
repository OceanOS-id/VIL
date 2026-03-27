// =============================================================================
// vil_trigger_cdc::events — connector events emitted on Data Lane
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when a row change is received from PostgreSQL logical replication.
#[connector_event]
pub struct RowChanged {
    /// FxHash of the table name.
    pub table_hash: u32,
    /// Operation type: 0=insert, 1=update, 2=delete.
    pub operation: u8,
    /// FxHash of the replication slot name.
    pub slot_hash: u32,
    /// PostgreSQL LSN (Log Sequence Number) truncated to u32 low bits.
    pub lsn_lo: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}
