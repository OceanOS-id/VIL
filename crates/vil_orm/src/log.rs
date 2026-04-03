//! VilORM db_log integration — emit semantic DB logs for every query.
//!
//! Called by VilEntity generated methods and VilQuery terminal methods.

/// Emit a db_log event for a VilEntity operation.
///
/// - `table`: table name (registered in dict for hash→string resolve)
/// - `sql`: SQL template string
/// - `duration_us`: query duration in microseconds
/// - `rows`: rows affected/returned
/// - `op_type`: 0=SELECT 1=INSERT 2=UPDATE 3=DELETE
/// - `error`: true if query failed
pub fn emit(table: &str, sql: &str, duration_us: u32, rows: u32, op_type: u8, error: bool) {
    let table_hash = vil_log::dict::register_str(table);
    let query_hash = vil_log::dict::register_str(sql);
    vil_log::db_log!(Info, vil_log::DbPayload {
        db_hash: 0,
        table_hash,
        query_hash,
        duration_us,
        rows_affected: rows,
        op_type,
        prepared: 1,
        tx_state: 0,
        error_code: if error { 1 } else { 0 },
        pool_id: 0,
        shard_id: 0,
        _pad: [0; 4],
        meta_bytes: [0; 160],
    });
}
