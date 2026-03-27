// =============================================================================
// vil_db_neo4j::client — Neo4jClient
// =============================================================================
//
// Neo4j graph client wrapper with VIL semantic log integration.
//
// - Every operation emits `db_log!` with timing and hash fields.
// - No println!, tracing::info!, or any non-VIL log call.
// - String fields use `register_str()` hashes.
// =============================================================================

use neo4rs::{Graph, ConfigBuilder};

use vil_log::{db_log, types::DbPayload};
use vil_log::dict::register_str;

use crate::config::Neo4jConfig;
use crate::error::Neo4jFault;
use crate::types::Neo4jResult;

/// Neo4j graph client wrapper with integrated VIL semantic logging.
///
/// Every operation automatically emits a `db_log!` entry with:
/// - `db_hash`       — FxHash of `"neo4j"`
/// - `query_hash`    — FxHash of the Cypher query string
/// - `duration_us`   — Wall-clock time of the operation
/// - `rows_affected` — Nodes/rows created/matched/returned
/// - `op_type`       — 0=MATCH 1=CREATE 4=CALL (transaction)
/// - `error_code`    — 0 on success, non-zero on fault
pub struct Neo4jClient {
    graph: Graph,
    /// FxHash of `"neo4j"` — cached for all log calls.
    db_hash: u32,
    /// Logical pool ID forwarded to DbPayload.
    pool_id: u16,
}

impl Neo4jClient {
    /// Connect to Neo4j and return a ready `Neo4jClient`.
    pub async fn new(config: Neo4jConfig) -> Neo4jResult<Self> {
        let uri_hash = register_str(&config.uri);
        let db_hash = register_str("neo4j");

        let neo4j_config = ConfigBuilder::default()
            .uri(&config.uri)
            .user(&config.user)
            .password(&config.password)
            .build()
            .map_err(|e| Neo4jFault::ConnectionFailed {
                uri_hash,
                reason_code: fault_code_from_err(&e),
            })?;

        let graph = Graph::connect(neo4j_config)
            .await
            .map_err(|e| Neo4jFault::ConnectionFailed {
                uri_hash,
                reason_code: fault_code_from_err(&e),
            })?;

        Ok(Self {
            graph,
            db_hash,
            pool_id: config.pool_id,
        })
    }

    /// Access the underlying `neo4rs::Graph`.
    pub fn raw_graph(&self) -> &Graph {
        &self.graph
    }

    /// Return the cached db_hash.
    pub fn db_hash(&self) -> u32 {
        self.db_hash
    }

    /// Return the pool_id.
    pub fn pool_id(&self) -> u16 {
        self.pool_id
    }
}

// =============================================================================
// Internal helper — emit a DbPayload log entry
// =============================================================================

/// Emit a `db_log!` entry for any Neo4j operation.
pub(crate) fn emit_db_log(
    db_hash: u32,
    query: &str,
    op_type: u8,
    duration_us: u32,
    rows_affected: u32,
    error_code: u8,
    pool_id: u16,
) {
    let query_hash = register_str(query);
    db_log!(Info, DbPayload {
        db_hash,
        query_hash,
        duration_us,
        rows_affected,
        op_type,
        error_code,
        pool_id,
        ..DbPayload::default()
    });
}

// =============================================================================
// Internal helper — stable numeric code from any error
// =============================================================================

pub(crate) fn fault_code_from_err<E: std::fmt::Debug>(e: &E) -> u32 {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut h = DefaultHasher::new();
    format!("{:?}", e).hash(&mut h);
    (h.finish() & 0xFFFF_FFFF) as u32
}
