// =============================================================================
// vil_log::types::db — DbPayload
// =============================================================================
//
// Database query/operation log payload.
// =============================================================================

/// Database operation event payload. Fits in 192 bytes.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DbPayload {
    /// FxHash of the database name.
    pub db_hash: u32,
    /// FxHash of the table/collection name.
    pub table_hash: u32,
    /// FxHash of the normalized query template.
    pub query_hash: u32,
    /// Query execution duration in microseconds.
    pub duration_us: u32,
    /// Number of rows affected/returned.
    pub rows_affected: u32,
    /// Operation type: 0=SELECT 1=INSERT 2=UPDATE 3=DELETE 4=CALL 5=DDL
    pub op_type: u8,
    /// Whether query was a prepared statement.
    pub prepared: u8,
    /// Transaction state: 0=none 1=begin 2=commit 3=rollback
    pub tx_state: u8,
    /// Error code (0 = success).
    pub error_code: u8,
    /// Connection pool ID.
    pub pool_id: u16,
    /// Shard/replica ID.
    pub shard_id: u16,
    /// Padding.
    pub _pad: [u8; 4],
    /// Inline query parameter metadata (msgpack).
    pub meta_bytes: [u8; 160],
}

impl Default for DbPayload {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

const _: () = {
    assert!(
        std::mem::size_of::<DbPayload>() <= 192,
        "DbPayload must fit within 192 bytes"
    );
};
