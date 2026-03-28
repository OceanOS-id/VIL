// =============================================================================
// vil_db_cassandra::client — CassandraClient
// =============================================================================
//
// ScyllaDB/Cassandra session wrapper with VIL semantic log integration.
//
// - Every operation emits `db_log!` with timing and hash fields.
// - No println!, tracing::info!, or any non-VIL log call.
// - String fields use `register_str()` hashes.
// =============================================================================

use scylla::{Session, SessionBuilder};

use vil_log::dict::register_str;
use vil_log::{db_log, types::DbPayload};

use crate::config::CassandraConfig;
use crate::error::CassandraFault;
use crate::types::CassandraResult;

/// Cassandra/ScyllaDB session wrapper with integrated VIL semantic logging.
///
/// Every operation automatically emits a `db_log!` entry with:
/// - `db_hash`       — FxHash of the keyspace name
/// - `query_hash`    — FxHash of the query string
/// - `duration_us`   — Wall-clock time of the operation
/// - `rows_affected` — Rows in the result set
/// - `op_type`       — 0=SELECT 1=INSERT 2=UPDATE 3=DELETE 4=BATCH 5=DDL
/// - `error_code`    — 0 on success, non-zero on fault
pub struct CassandraClient {
    session: Session,
    /// FxHash of the keyspace name — cached to avoid re-hashing on every call.
    db_hash: u32,
    /// Logical pool ID forwarded to DbPayload.
    pool_id: u16,
}

impl CassandraClient {
    /// Connect to Cassandra/ScyllaDB and return a ready `CassandraClient`.
    pub async fn new(config: CassandraConfig) -> CassandraResult<Self> {
        let uri_hash = register_str(&config.contact_points.join(","));
        let db_hash = register_str(&config.keyspace);

        let mut builder = SessionBuilder::new();
        for cp in &config.contact_points {
            builder = builder.known_node(cp.as_str());
        }
        builder = builder.use_keyspace(&config.keyspace, false);

        let session = builder
            .build()
            .await
            .map_err(|e| CassandraFault::ConnectionFailed {
                uri_hash,
                reason_code: fault_code_from_err(&e),
            })?;

        Ok(Self {
            session,
            db_hash,
            pool_id: config.pool_id,
        })
    }

    /// Access the underlying `scylla::Session`.
    pub fn raw_session(&self) -> &Session {
        &self.session
    }

    /// Return the cached db_hash (FxHash of keyspace).
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

/// Emit a `db_log!` entry for any Cassandra operation.
pub(crate) fn emit_db_log(
    db_hash: u32,
    query: &str,
    op_type: u8,
    prepared: u8,
    duration_us: u32,
    rows_affected: u32,
    error_code: u8,
    pool_id: u16,
) {
    let query_hash = register_str(query);
    db_log!(
        Info,
        DbPayload {
            db_hash,
            query_hash,
            duration_us,
            rows_affected,
            op_type,
            prepared,
            error_code,
            pool_id,
            ..DbPayload::default()
        }
    );
}

// =============================================================================
// Internal helper — stable numeric code from any error
// =============================================================================

pub(crate) fn fault_code_from_err<E: std::fmt::Debug>(e: &E) -> u32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    format!("{:?}", e).hash(&mut h);
    (h.finish() & 0xFFFF_FFFF) as u32
}
