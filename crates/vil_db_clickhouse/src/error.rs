// =============================================================================
// vil_db_clickhouse::error — ChFault
// =============================================================================
//
// Semantic fault type for ClickHouse operations.
// All string fields are stored as u32 hashes via register_str().
// No heap types — compliant with vil_fault layout requirements.
// =============================================================================

/// ClickHouse operation fault type.
///
/// All string identifiers (URL, table, query, operation) are stored as
/// 32-bit FxHash values computed via `vil_log::dict::register_str()`.
/// This keeps the error type allocation-free on the hot path.
#[derive(Debug, Clone, Copy)]
pub enum ChFault {
    /// TCP/HTTP connection to ClickHouse failed.
    ConnectionFailed {
        /// FxHash of the ClickHouse URL.
        url_hash: u32,
        /// Underlying error code (OS errno or HTTP status).
        reason_code: u32,
    },

    /// A SELECT/DDL query returned an error from ClickHouse.
    QueryFailed {
        /// FxHash of the SQL query string.
        query_hash: u32,
        /// ClickHouse error code or HTTP status.
        reason_code: u32,
    },

    /// A batch or single-row INSERT failed.
    InsertFailed {
        /// FxHash of the target table name.
        table_hash: u32,
        /// Number of rows that were attempted.
        rows: u64,
        /// ClickHouse error code or HTTP status.
        reason_code: u32,
    },

    /// An async operation exceeded its deadline.
    Timeout {
        /// FxHash of the operation label (e.g., "query", "insert", "flush").
        operation_hash: u32,
        /// Elapsed time in milliseconds before the timeout fired.
        elapsed_ms: u32,
    },
}

impl std::fmt::Display for ChFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChFault::ConnectionFailed { url_hash, reason_code } => {
                write!(f, "ChFault::ConnectionFailed(url={url_hash:#010x}, code={reason_code})")
            }
            ChFault::QueryFailed { query_hash, reason_code } => {
                write!(f, "ChFault::QueryFailed(query={query_hash:#010x}, code={reason_code})")
            }
            ChFault::InsertFailed { table_hash, rows, reason_code } => {
                write!(f, "ChFault::InsertFailed(table={table_hash:#010x}, rows={rows}, code={reason_code})")
            }
            ChFault::Timeout { operation_hash, elapsed_ms } => {
                write!(f, "ChFault::Timeout(op={operation_hash:#010x}, elapsed={elapsed_ms}ms)")
            }
        }
    }
}

impl std::error::Error for ChFault {}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Extract a numeric reason code from a clickhouse::error::Error.
/// Maps to an integer so ChFault stays allocation-free.
pub(crate) fn clickhouse_error_code(e: &clickhouse::error::Error) -> u32 {
    // clickhouse::error::Error is non-exhaustive; use its Display hash as a
    // stable numeric stand-in when no explicit integer code is available.
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    e.to_string().hash(&mut h);
    (h.finish() & 0xFFFF_FFFF) as u32
}
