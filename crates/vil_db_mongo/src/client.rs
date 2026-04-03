// =============================================================================
// vil_db_mongo::client — MongoClient
// =============================================================================
//
// MongoDB client wrapper with VIL semantic log integration.
//
// - Every CRUD operation emits `db_log!` with timing and hash fields.
// - No println!, tracing::info!, or log::info! — COMPLIANCE.md §8.
// - String fields use `register_str()` hashes — no raw strings on hot path.
// - Connection pool configured from MongoConfig at construction.
// =============================================================================

use mongodb::options::ClientOptions;

use vil_log::dict::register_str;
use vil_log::{db_log, types::DbPayload};

use crate::config::MongoConfig;
use crate::error::MongoFault;
use crate::types::MongoResult;

/// MongoDB client wrapper with integrated VIL semantic logging.
///
/// Every operation automatically emits a `db_log!` entry with:
/// - `db_hash`        — FxHash of the database name
/// - `table_hash`     — FxHash of the collection name
/// - `duration_ns`    — Wall-clock time of the operation
/// - `rows_affected`  — Documents matched/modified/inserted
/// - `op_type`        — 0=SELECT, 1=INSERT, 2=UPDATE, 3=DELETE
/// - `error_code`     — 0 on success, non-zero on fault
pub struct MongoClient {
    inner: mongodb::Client,
    db: mongodb::Database,
    /// FxHash of the database name — cached to avoid re-hashing on every call.
    db_hash: u32,
}

impl MongoClient {
    /// Connect to MongoDB and return a ready `MongoClient`.
    pub async fn new(config: MongoConfig) -> MongoResult<Self> {
        let uri_hash = register_str(&config.uri);
        let db_hash = register_str(&config.database);

        let mut opts =
            ClientOptions::parse(&config.uri)
                .await
                .map_err(|e| MongoFault::ConnectionFailed {
                    uri_hash,
                    reason_code: fault_code_from_mongo_err(&e),
                })?;

        if let Some(min) = config.min_pool {
            opts.min_pool_size = Some(min);
        }
        if let Some(max) = config.max_pool {
            opts.max_pool_size = Some(max);
        }

        let inner =
            mongodb::Client::with_options(opts).map_err(|e| MongoFault::ConnectionFailed {
                uri_hash,
                reason_code: fault_code_from_mongo_err(&e),
            })?;

        let db = inner.database(&config.database);

        Ok(Self { inner, db, db_hash })
    }

    /// Access the inner `mongodb::Client` for advanced use.
    pub fn raw_client(&self) -> &mongodb::Client {
        &self.inner
    }

    /// Access the target `mongodb::Database` directly.
    pub fn raw_db(&self) -> &mongodb::Database {
        &self.db
    }

    /// Return the FxHash of the configured database name.
    pub fn db_hash(&self) -> u32 {
        self.db_hash
    }
}

// =============================================================================
// Internal helper — deterministic numeric code from a mongodb error
// =============================================================================

pub(crate) fn fault_code_from_mongo_err(e: &mongodb::error::Error) -> u32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    format!("{:?}", e.kind).hash(&mut h);
    (h.finish() & 0xFFFF_FFFF) as u32
}

// =============================================================================
// Internal log helper used by crud.rs
// =============================================================================

/// Emit a `db_log!` entry after any operation, on both success and failure.
pub(crate) fn emit_db_log(
    db_hash: u32,
    collection: &str,
    op_type: u8,
    duration_ns: u64,
    rows_affected: u32,
    error_code: u8,
) {
    let table_hash = register_str(collection);
    db_log!(
        Info,
        DbPayload {
            db_hash,
            table_hash,
            duration_ns,
            rows_affected,
            op_type,
            error_code,
            ..DbPayload::default()
        }
    );
}
