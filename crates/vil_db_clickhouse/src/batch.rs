// =============================================================================
// vil_db_clickhouse::batch — BatchInserter
// =============================================================================
//
// Buffered batch inserter with two flush policies:
//   - max_rows : flush when buffer reaches this many rows
//   - max_wait : flush when the oldest buffered row has waited this long
//
// `push()` appends a row and auto-flushes if either policy triggers.
// `flush()` drains the buffer unconditionally and emits db_log! with the
//   full batch size as `rows_affected`.
//
// The caller is responsible for calling `flush()` at shutdown to drain
// any remaining rows.
// =============================================================================

use std::time::{Duration, Instant};

use clickhouse::Row;
use serde::Serialize;
use vil_log::dict::register_str;
use vil_log::{db_log, DbPayload};

use crate::client::ChClient;
use crate::error::{clickhouse_error_code, ChFault};

// ---------------------------------------------------------------------------
// BatchInserter
// ---------------------------------------------------------------------------

/// Buffered ClickHouse inserter that auto-flushes by row count or time.
///
/// # Flush policy
///
/// A flush is triggered automatically inside `push()` when **either**:
/// - The buffer reaches `max_rows` entries, **or**
/// - `max_wait` has elapsed since `last_flush`.
///
/// Call `flush()` explicitly at process shutdown to drain remaining rows.
///
/// # Type parameter
///
/// `T` must implement `clickhouse::Row + serde::Serialize`. Rows are moved
/// into the internal `Vec<T>` buffer; no heap allocation occurs on the
/// ClickHouse wire path beyond the vec itself.
pub struct BatchInserter<T: Row + Serialize> {
    client: ChClient,
    table: String,
    /// FxHash of `table`, computed once at construction.
    table_hash: u32,
    buffer: Vec<T>,
    max_rows: usize,
    max_wait: Duration,
    last_flush: Instant,
}

impl<T: Row + Serialize> BatchInserter<T> {
    /// Create a new `BatchInserter`.
    ///
    /// - `client`   — a `ChClient` to use for flushing.
    /// - `table`    — ClickHouse table name for INSERT.
    /// - `max_rows` — flush when the buffer reaches this many rows.
    /// - `max_wait` — flush when the oldest row has been buffered this long.
    pub fn new(client: ChClient, table: &str, max_rows: usize, max_wait: Duration) -> Self {
        let table_hash = register_str(table);
        Self {
            client,
            table: table.to_owned(),
            table_hash,
            buffer: Vec::with_capacity(max_rows),
            max_rows,
            max_wait,
            last_flush: Instant::now(),
        }
    }

    // -----------------------------------------------------------------------
    // push — buffer a row, auto-flush if policy triggers
    // -----------------------------------------------------------------------

    /// Append `row` to the buffer and flush if a policy threshold is met.
    ///
    /// Auto-flush triggers when:
    /// - `buffer.len() >= max_rows`, or
    /// - time since last flush >= `max_wait`.
    pub async fn push(&mut self, row: T) -> Result<(), ChFault> {
        self.buffer.push(row);

        let should_flush =
            self.buffer.len() >= self.max_rows || self.last_flush.elapsed() >= self.max_wait;

        if should_flush {
            self.flush().await?;
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // flush — drain the buffer into ClickHouse
    // -----------------------------------------------------------------------

    /// Flush all buffered rows to ClickHouse via a single batch INSERT.
    ///
    /// Emits `db_log!(Info/Error, ...)` with `rows_affected = buffer.len()`.
    /// The buffer is cleared on both success and error to avoid re-sending
    /// duplicate rows on retry.
    ///
    /// Returns the number of rows that were flushed.
    pub async fn flush(&mut self) -> Result<u64, ChFault> {
        if self.buffer.is_empty() {
            return Ok(0);
        }

        let start = Instant::now();
        let row_count = self.buffer.len() as u64;

        // Drain the buffer before the async call so we never double-flush
        // even if the caller ignores the error and calls flush() again.
        let rows: Vec<T> = self.buffer.drain(..).collect();
        self.last_flush = Instant::now();

        let result: Result<(), clickhouse::error::Error> = async {
            let mut ins = self.client.inner.insert(&self.table)?;
            for row in &rows {
                ins.write(row).await?;
            }
            ins.end().await?;
            Ok(())
        }
        .await;

        let elapsed_us = start.elapsed().as_micros() as u32;

        match result {
            Ok(()) => {
                db_log!(
                    Info,
                    DbPayload {
                        db_hash: self.client.db_hash,
                        table_hash: self.table_hash,
                        query_hash: 0,
                        duration_us: elapsed_us,
                        rows_affected: row_count as u32,
                        op_type: 1, // INSERT
                        error_code: 0,
                        ..DbPayload::default()
                    }
                );
                Ok(row_count)
            }
            Err(e) => {
                let reason_code = clickhouse_error_code(&e);
                db_log!(
                    Error,
                    DbPayload {
                        db_hash: self.client.db_hash,
                        table_hash: self.table_hash,
                        query_hash: 0,
                        duration_us: elapsed_us,
                        rows_affected: 0,
                        op_type: 1, // INSERT
                        error_code: (reason_code & 0xFF) as u8,
                        ..DbPayload::default()
                    }
                );
                Err(ChFault::InsertFailed {
                    table_hash: self.table_hash,
                    rows: row_count,
                    reason_code,
                })
            }
        }
    }

    // -----------------------------------------------------------------------
    // Accessors (useful for tests and metrics)
    // -----------------------------------------------------------------------

    /// Number of rows currently waiting in the buffer.
    #[inline]
    pub fn buffered_rows(&self) -> usize {
        self.buffer.len()
    }

    /// Time elapsed since the last successful flush.
    #[inline]
    pub fn time_since_flush(&self) -> Duration {
        self.last_flush.elapsed()
    }
}
