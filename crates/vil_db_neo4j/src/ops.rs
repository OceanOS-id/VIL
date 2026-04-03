// =============================================================================
// vil_db_neo4j::ops — Neo4j operations on Neo4jClient
// =============================================================================
//
// All operations:
//   1. Record `Instant::now()` before the driver call.
//   2. Execute the driver call.
//   3. Emit `db_log!` via `emit_db_log` with timing, op_type, and error_code.
//   4. Return Result<T, Neo4jFault>.
//
// op_type constants:
//   0 = MATCH  (execute, match_query)
//   1 = CREATE (create_node)
//   4 = CALL   (run_transaction)
//
// No println!, tracing::info!, or any non-VIL log call.
//
// Note: neo4rs::Graph::execute() returns DetachedRowStream.
//       neo4rs::Txn::execute() returns RowStream (requires txn handle to advance).
// =============================================================================

use std::time::Instant;

use neo4rs::Row;

use vil_log::dict::register_str;

use crate::client::{emit_db_log, fault_code_from_err, Neo4jClient};
use crate::error::Neo4jFault;
use crate::types::Neo4jResult;

// op_type codes
const OP_MATCH: u8 = 0;
const OP_CREATE: u8 = 1;
const OP_CALL: u8 = 4;

impl Neo4jClient {
    // =========================================================================
    // execute
    // =========================================================================

    /// Execute a Cypher query string and collect all returned rows.
    ///
    /// `cypher` is the raw Cypher string.
    /// Emits `db_log!` with `op_type = 0` (MATCH/SELECT).
    pub async fn execute(&self, cypher: &str) -> Neo4jResult<Vec<Row>> {
        let query_hash = register_str(cypher);

        let start = Instant::now();
        let stream_result = self.raw_graph().execute(neo4rs::query(cypher)).await;
        let elapsed_ns = start.elapsed().as_nanos() as u64;

        let mut stream = match stream_result {
            Ok(s) => s,
            Err(e) => {
                emit_db_log(
                    self.db_hash(),
                    cypher,
                    OP_MATCH,
                    elapsed_ns,
                    0,
                    1,
                    self.pool_id(),
                );
                return Err(Neo4jFault::ExecuteFailed {
                    query_hash,
                    reason_code: fault_code_from_err(&e),
                });
            }
        };

        let mut rows = Vec::new();
        loop {
            match stream.next().await {
                Ok(Some(row)) => rows.push(row),
                Ok(None) => break,
                Err(e) => {
                    let total_ns = start.elapsed().as_nanos() as u64;
                    emit_db_log(
                        self.db_hash(),
                        cypher,
                        OP_MATCH,
                        total_ns,
                        0,
                        1,
                        self.pool_id(),
                    );
                    return Err(Neo4jFault::ExecuteFailed {
                        query_hash,
                        reason_code: fault_code_from_err(&e),
                    });
                }
            }
        }

        let total_ns = start.elapsed().as_nanos() as u64;
        let count = rows.len() as u32;
        emit_db_log(
            self.db_hash(),
            cypher,
            OP_MATCH,
            total_ns,
            count,
            0,
            self.pool_id(),
        );
        Ok(rows)
    }

    // =========================================================================
    // run_transaction
    // =========================================================================

    /// Run a Cypher statement inside a transaction, then commit.
    ///
    /// Emits `db_log!` with `op_type = 4` (CALL / transaction).
    pub async fn run_transaction(&self, cypher: &str) -> Neo4jResult<()> {
        let _query_hash = register_str(cypher);

        let start = Instant::now();
        let txn_result = self.raw_graph().start_txn().await;

        let mut txn = match txn_result {
            Ok(t) => t,
            Err(e) => {
                let elapsed_ns = start.elapsed().as_nanos() as u64;
                emit_db_log(
                    self.db_hash(),
                    cypher,
                    OP_CALL,
                    elapsed_ns,
                    0,
                    1,
                    self.pool_id(),
                );
                return Err(Neo4jFault::TransactionFailed {
                    reason_code: fault_code_from_err(&e),
                });
            }
        };

        let run_result = txn.run(neo4rs::query(cypher)).await;
        if let Err(e) = run_result {
            let elapsed_ns = start.elapsed().as_nanos() as u64;
            let _ = txn.rollback().await;
            emit_db_log(
                self.db_hash(),
                cypher,
                OP_CALL,
                elapsed_ns,
                0,
                1,
                self.pool_id(),
            );
            return Err(Neo4jFault::TransactionFailed {
                reason_code: fault_code_from_err(&e),
            });
        }

        let commit_result = txn.commit().await;
        let elapsed_ns = start.elapsed().as_nanos() as u64;

        match commit_result {
            Ok(_) => {
                emit_db_log(
                    self.db_hash(),
                    cypher,
                    OP_CALL,
                    elapsed_ns,
                    0,
                    0,
                    self.pool_id(),
                );
                Ok(())
            }
            Err(e) => {
                emit_db_log(
                    self.db_hash(),
                    cypher,
                    OP_CALL,
                    elapsed_ns,
                    0,
                    1,
                    self.pool_id(),
                );
                Err(Neo4jFault::TransactionFailed {
                    reason_code: fault_code_from_err(&e),
                })
            }
        }
    }

    // =========================================================================
    // create_node
    // =========================================================================

    /// Create a node with the given `label` and property Cypher literal.
    ///
    /// `props_cypher` is the Cypher property map literal, e.g.
    /// `"{ name: 'Alice', age: 30 }"`.
    ///
    /// Emits `db_log!` with `op_type = 1` (CREATE).
    pub async fn create_node(&self, label: &str, props_cypher: &str) -> Neo4jResult<()> {
        let label_hash = register_str(label);
        let cypher = format!("CREATE (n:{} {})", label, props_cypher);

        let start = Instant::now();
        let result = self.raw_graph().run(neo4rs::query(&cypher)).await;
        let elapsed_ns = start.elapsed().as_nanos() as u64;

        match result {
            Ok(_) => {
                emit_db_log(
                    self.db_hash(),
                    &cypher,
                    OP_CREATE,
                    elapsed_ns,
                    1,
                    0,
                    self.pool_id(),
                );
                Ok(())
            }
            Err(e) => {
                emit_db_log(
                    self.db_hash(),
                    &cypher,
                    OP_CREATE,
                    elapsed_ns,
                    0,
                    1,
                    self.pool_id(),
                );
                Err(Neo4jFault::CreateNodeFailed {
                    label_hash,
                    reason_code: fault_code_from_err(&e),
                })
            }
        }
    }

    // =========================================================================
    // match_query
    // =========================================================================

    /// Run a MATCH Cypher query and collect all rows.
    ///
    /// Returns a `Vec<neo4rs::Row>`.
    /// Emits `db_log!` with `op_type = 0` (MATCH).
    pub async fn match_query(&self, cypher: &str) -> Neo4jResult<Vec<Row>> {
        let query_hash = register_str(cypher);

        let start = Instant::now();
        let stream_result = self.raw_graph().execute(neo4rs::query(cypher)).await;
        let elapsed_ns = start.elapsed().as_nanos() as u64;

        let mut stream = match stream_result {
            Ok(s) => s,
            Err(e) => {
                emit_db_log(
                    self.db_hash(),
                    cypher,
                    OP_MATCH,
                    elapsed_ns,
                    0,
                    1,
                    self.pool_id(),
                );
                return Err(Neo4jFault::MatchFailed {
                    query_hash,
                    reason_code: fault_code_from_err(&e),
                });
            }
        };

        let mut rows = Vec::new();
        loop {
            match stream.next().await {
                Ok(Some(row)) => rows.push(row),
                Ok(None) => break,
                Err(e) => {
                    let total_ns = start.elapsed().as_nanos() as u64;
                    emit_db_log(
                        self.db_hash(),
                        cypher,
                        OP_MATCH,
                        total_ns,
                        0,
                        1,
                        self.pool_id(),
                    );
                    return Err(Neo4jFault::MatchFailed {
                        query_hash,
                        reason_code: fault_code_from_err(&e),
                    });
                }
            }
        }

        let total_ns = start.elapsed().as_nanos() as u64;
        let count = rows.len() as u32;
        emit_db_log(
            self.db_hash(),
            cypher,
            OP_MATCH,
            total_ns,
            count,
            0,
            self.pool_id(),
        );
        Ok(rows)
    }
}
