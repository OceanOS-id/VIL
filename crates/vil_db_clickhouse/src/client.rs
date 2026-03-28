// =============================================================================
// vil_db_clickhouse::client — ChClient
// =============================================================================
//
// Core ClickHouse client wrapping `clickhouse::Client`.
//
// Every public operation:
//   1. Records `Instant::now()` at entry.
//   2. Executes the ClickHouse call.
//   3. Emits `db_log!(Info/Error, DbPayload { ... })` with elapsed_us before
//      returning — regardless of success or failure.
//
// op_type codes (DbPayload::op_type):
//   0 = SELECT  1 = INSERT  2 = UPDATE  3 = DELETE  4 = CALL  5 = DDL
// =============================================================================

use std::time::Instant;

use clickhouse::Row;
use serde::{Deserialize, Serialize};
use vil_log::dict::register_str;
use vil_log::{db_log, DbPayload};

use crate::config::ClickHouseConfig;
use crate::error::{clickhouse_error_code, ChFault};

// ---------------------------------------------------------------------------
// ChClient
// ---------------------------------------------------------------------------

/// Async ClickHouse client with `db_log!` auto-emit on every operation.
///
/// Thread-safe — `clickhouse::Client` is `Clone + Send + Sync`.
///
/// # Thread hint
/// `ChClient` itself holds no background threads. The underlying HTTP pool
/// managed by `clickhouse::Client` uses tokio tasks. Add those counts to
/// your `LogConfig.threads` budget when sizing the log ring.
#[derive(Clone)]
pub struct ChClient {
    pub(crate) inner: clickhouse::Client,
    /// FxHash of `config.database`, pre-computed at construction time.
    pub(crate) db_hash: u32,
}

impl ChClient {
    /// Build a new `ChClient` from the provided configuration.
    ///
    /// No network I/O is performed here; the first actual connection attempt
    /// happens on the first query or insert.
    pub fn new(config: ClickHouseConfig) -> Self {
        let db_hash = register_str(&config.database);

        let mut client = clickhouse::Client::default()
            .with_url(&config.url)
            .with_database(&config.database);

        if let Some(ref user) = config.username {
            client = client.with_user(user);
        }
        if let Some(ref pass) = config.password {
            client = client.with_password(pass);
        }

        Self {
            inner: client,
            db_hash,
        }
    }

    // -----------------------------------------------------------------------
    // SELECT — returns Vec<T>
    // -----------------------------------------------------------------------

    /// Execute a SELECT query and collect all rows into a `Vec<T>`.
    ///
    /// Emits `db_log!(Info, ...)` with `op_type = 0` (SELECT) and elapsed
    /// microseconds. On error, emits `db_log!(Error, ...)` with the fault's
    /// reason code in `error_code`.
    pub async fn query<T>(&self, sql: &str) -> Result<Vec<T>, ChFault>
    where
        T: Row + for<'de> Deserialize<'de>,
    {
        let start = Instant::now();
        let query_hash = register_str(sql);

        let result = self.inner.query(sql).fetch_all::<T>().await;

        let elapsed_us = start.elapsed().as_micros() as u32;

        match result {
            Ok(rows) => {
                db_log!(
                    Info,
                    DbPayload {
                        db_hash: self.db_hash,
                        table_hash: 0,
                        query_hash,
                        duration_us: elapsed_us,
                        rows_affected: rows.len() as u32,
                        op_type: 0, // SELECT
                        error_code: 0,
                        ..DbPayload::default()
                    }
                );
                Ok(rows)
            }
            Err(e) => {
                let reason_code = clickhouse_error_code(&e);
                db_log!(
                    Error,
                    DbPayload {
                        db_hash: self.db_hash,
                        table_hash: 0,
                        query_hash,
                        duration_us: elapsed_us,
                        rows_affected: 0,
                        op_type: 0, // SELECT
                        error_code: (reason_code & 0xFF) as u8,
                        ..DbPayload::default()
                    }
                );
                Err(ChFault::QueryFailed {
                    query_hash,
                    reason_code,
                })
            }
        }
    }

    // -----------------------------------------------------------------------
    // DDL / fire-and-forget execute
    // -----------------------------------------------------------------------

    /// Execute a DDL statement or any query that returns no rows.
    ///
    /// Emits `db_log!(Info/Error, ...)` with `op_type = 5` (DDL).
    pub async fn execute(&self, sql: &str) -> Result<(), ChFault> {
        let start = Instant::now();
        let query_hash = register_str(sql);

        let result = self.inner.query(sql).execute().await;

        let elapsed_us = start.elapsed().as_micros() as u32;

        match result {
            Ok(()) => {
                db_log!(
                    Info,
                    DbPayload {
                        db_hash: self.db_hash,
                        table_hash: 0,
                        query_hash,
                        duration_us: elapsed_us,
                        rows_affected: 0,
                        op_type: 5, // DDL
                        error_code: 0,
                        ..DbPayload::default()
                    }
                );
                Ok(())
            }
            Err(e) => {
                let reason_code = clickhouse_error_code(&e);
                db_log!(
                    Error,
                    DbPayload {
                        db_hash: self.db_hash,
                        table_hash: 0,
                        query_hash,
                        duration_us: elapsed_us,
                        rows_affected: 0,
                        op_type: 5, // DDL
                        error_code: (reason_code & 0xFF) as u8,
                        ..DbPayload::default()
                    }
                );
                Err(ChFault::QueryFailed {
                    query_hash,
                    reason_code,
                })
            }
        }
    }

    // -----------------------------------------------------------------------
    // Batch INSERT
    // -----------------------------------------------------------------------

    /// INSERT a slice of rows into `table` in a single ClickHouse batch.
    ///
    /// Returns the number of rows inserted on success.
    /// Emits `db_log!(Info/Error, ...)` with `op_type = 1` (INSERT).
    pub async fn insert<T>(&self, table: &str, rows: &[T]) -> Result<u64, ChFault>
    where
        T: Row + Serialize,
    {
        let start = Instant::now();
        let table_hash = register_str(table);
        let row_count = rows.len() as u64;

        let result: Result<(), clickhouse::error::Error> = async {
            let mut ins = self.inner.insert(table)?;
            for row in rows {
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
                        db_hash: self.db_hash,
                        table_hash,
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
                        db_hash: self.db_hash,
                        table_hash,
                        query_hash: 0,
                        duration_us: elapsed_us,
                        rows_affected: 0,
                        op_type: 1, // INSERT
                        error_code: (reason_code & 0xFF) as u8,
                        ..DbPayload::default()
                    }
                );
                Err(ChFault::InsertFailed {
                    table_hash,
                    rows: row_count,
                    reason_code,
                })
            }
        }
    }
}
