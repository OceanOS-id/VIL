// =============================================================================
// vil_db_cassandra::ops — Cassandra/ScyllaDB operations on CassandraClient
// =============================================================================
//
// All operations:
//   1. Record `Instant::now()` before the driver call.
//   2. Execute the driver call.
//   3. Emit `db_log!` via `emit_db_log` with timing, op_type, and error_code.
//   4. Return Result<T, CassandraFault>.
//
// op_type constants:
//   0 = SELECT (query, execute, execute_paged)
//   4 = BATCH
//
// prepared flag: 1 = prepared statement, 0 = ad-hoc
//
// No println!, tracing::info!, or any non-VIL log call.
//
// scylla 0.14 API notes:
//   - session.query_unpaged(cql, values)        -> ad-hoc, no paging
//   - session.execute_unpaged(prepared, values) -> prepared, no paging
//   - session.execute_single_page(prepared, values, paging_state) -> one page
//   - session.batch(batch, values)              -> batch execution
//   - PagingState is at scylla::statement::PagingState
//   - BatchValues is at scylla::serialize::batch::BatchValues
//   - SerializeRow is at scylla::SerializeRow (via macros re-export)
// =============================================================================

use std::time::Instant;

use scylla::prepared_statement::PreparedStatement;
use scylla::batch::Batch;
use scylla::QueryResult;
use scylla::frame::response::result::Row;
use scylla::statement::PagingState;
use scylla::serialize::batch::BatchValues;
use scylla::serialize::row::SerializeRow;

use vil_log::dict::register_str;

use crate::client::{emit_db_log, fault_code_from_err, CassandraClient};
use crate::error::CassandraFault;
use crate::types::CassandraResult;

// op_type codes
const OP_SELECT: u8 = 0;
const OP_BATCH: u8  = 4;

impl CassandraClient {
    // =========================================================================
    // prepare
    // =========================================================================

    /// Prepare a CQL statement for reuse.
    ///
    /// Returns the `PreparedStatement` on success.
    /// Does NOT emit a db_log (setup-time operation, not a query).
    pub async fn prepare(&self, cql: &str) -> CassandraResult<PreparedStatement> {
        let query_hash = register_str(cql);
        self.raw_session()
            .prepare(cql)
            .await
            .map_err(|e| CassandraFault::PrepareFailed {
                query_hash,
                reason_code: fault_code_from_err(&e),
            })
    }

    // =========================================================================
    // execute (prepared, unpaged)
    // =========================================================================

    /// Execute a prepared statement (no paging).
    ///
    /// Emits `db_log!` with `op_type = 0` (SELECT), `prepared = 1`.
    pub async fn execute(
        &self,
        prepared: &PreparedStatement,
        values: impl SerializeRow,
    ) -> CassandraResult<QueryResult> {
        let cql = prepared.get_statement();
        let query_hash = register_str(cql);

        let start = Instant::now();
        let result = self.raw_session().execute_unpaged(prepared, values).await;
        let elapsed_us = start.elapsed().as_micros() as u32;

        match result {
            Ok(qr) => {
                let rows = qr.rows.as_ref().map(|r| r.len() as u32).unwrap_or(0);
                emit_db_log(self.db_hash(), cql, OP_SELECT, 1, elapsed_us, rows, 0, self.pool_id());
                Ok(qr)
            }
            Err(e) => {
                emit_db_log(self.db_hash(), cql, OP_SELECT, 1, elapsed_us, 0, 1, self.pool_id());
                Err(CassandraFault::ExecuteFailed {
                    query_hash,
                    reason_code: fault_code_from_err(&e),
                })
            }
        }
    }

    // =========================================================================
    // query (ad-hoc, unpaged)
    // =========================================================================

    /// Execute an ad-hoc CQL query (no paging).
    ///
    /// Emits `db_log!` with `op_type = 0` (SELECT), `prepared = 0`.
    pub async fn query(
        &self,
        cql: &str,
        values: impl SerializeRow,
    ) -> CassandraResult<QueryResult> {
        let query_hash = register_str(cql);

        let start = Instant::now();
        let result = self.raw_session().query_unpaged(cql, values).await;
        let elapsed_us = start.elapsed().as_micros() as u32;

        match result {
            Ok(qr) => {
                let rows = qr.rows.as_ref().map(|r| r.len() as u32).unwrap_or(0);
                emit_db_log(self.db_hash(), cql, OP_SELECT, 0, elapsed_us, rows, 0, self.pool_id());
                Ok(qr)
            }
            Err(e) => {
                emit_db_log(self.db_hash(), cql, OP_SELECT, 0, elapsed_us, 0, 1, self.pool_id());
                Err(CassandraFault::QueryFailed {
                    query_hash,
                    reason_code: fault_code_from_err(&e),
                })
            }
        }
    }

    // =========================================================================
    // batch
    // =========================================================================

    /// Execute a `Batch` of statements.
    ///
    /// Emits `db_log!` with `op_type = 4` (BATCH).
    pub async fn batch(
        &self,
        batch: &Batch,
        values: impl BatchValues,
    ) -> CassandraResult<QueryResult> {
        let start = Instant::now();
        let result = self.raw_session().batch(batch, values).await;
        let elapsed_us = start.elapsed().as_micros() as u32;

        match result {
            Ok(qr) => {
                emit_db_log(self.db_hash(), "batch", OP_BATCH, 0, elapsed_us, 0, 0, self.pool_id());
                Ok(qr)
            }
            Err(e) => {
                emit_db_log(self.db_hash(), "batch", OP_BATCH, 0, elapsed_us, 0, 1, self.pool_id());
                Err(CassandraFault::BatchFailed {
                    reason_code: fault_code_from_err(&e),
                })
            }
        }
    }

    // =========================================================================
    // execute_paged
    // =========================================================================

    /// Execute a prepared statement with manual paging, collecting all rows.
    ///
    /// Iterates through all pages using `execute_single_page`.
    /// Emits one `db_log!` per page with `op_type = 0`, `prepared = 1`.
    pub async fn execute_paged(
        &self,
        prepared: &PreparedStatement,
        values: impl SerializeRow + Clone,
    ) -> CassandraResult<Vec<Row>> {
        let cql = prepared.get_statement();
        let query_hash = register_str(cql);

        let mut paging_state = PagingState::start();
        let mut all_rows: Vec<Row> = Vec::new();

        loop {
            let start = Instant::now();
            let result = self
                .raw_session()
                .execute_single_page(prepared, values.clone(), paging_state)
                .await;
            let elapsed_us = start.elapsed().as_micros() as u32;

            match result {
                Ok((qr, paging_state_response)) => {
                    let page_rows = qr.rows.unwrap_or_default();
                    let count = page_rows.len() as u32;
                    emit_db_log(self.db_hash(), cql, OP_SELECT, 1, elapsed_us, count, 0, self.pool_id());
                    all_rows.extend(page_rows);

                    use std::ops::ControlFlow;
                    match paging_state_response.into_paging_control_flow() {
                        ControlFlow::Break(()) => break,
                        ControlFlow::Continue(next_state) => {
                            paging_state = next_state;
                        }
                    }
                }
                Err(e) => {
                    emit_db_log(self.db_hash(), cql, OP_SELECT, 1, elapsed_us, 0, 1, self.pool_id());
                    return Err(CassandraFault::PagedFailed {
                        query_hash,
                        reason_code: fault_code_from_err(&e),
                    });
                }
            }
        }

        Ok(all_rows)
    }
}
