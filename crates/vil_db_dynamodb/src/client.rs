// =============================================================================
// vil_db_dynamodb::client — DynamoClient
// =============================================================================
//
// AWS DynamoDB client wrapper with VIL semantic log integration.
//
// - Every operation emits `db_log!` with timing and hash fields.
// - No println!, tracing::info!, or any non-VIL log call.
// - String fields use `register_str()` hashes.
// =============================================================================

use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::config::Builder as DynConfigBuilder;
use aws_sdk_dynamodb::Client;

use vil_log::dict::register_str;
use vil_log::{db_log, types::DbPayload};

use crate::config::DynamoConfig;
use crate::types::DynamoResult;

/// DynamoDB client wrapper with integrated VIL semantic logging.
///
/// Every operation automatically emits a `db_log!` entry with:
/// - `db_hash`       — FxHash of `"dynamodb"`
/// - `table_hash`    — FxHash of the table name
/// - `duration_us`   — Wall-clock time of the operation
/// - `rows_affected` — Items returned/affected
/// - `op_type`       — 0=GET 1=PUT 2=UPDATE 3=DELETE 4=QUERY 5=SCAN
/// - `error_code`    — 0 on success, non-zero on fault
pub struct DynamoClient {
    inner: Client,
    /// FxHash of `"dynamodb"` — cached for all log calls.
    db_hash: u32,
    /// Logical pool ID forwarded to DbPayload.
    pool_id: u16,
}

impl DynamoClient {
    /// Build a `DynamoClient` from `DynamoConfig`.
    ///
    /// Loads AWS credentials from the environment / instance profile.
    pub async fn new(config: DynamoConfig) -> DynamoResult<Self> {
        let db_hash = register_str("dynamodb");

        let region_provider = RegionProviderChain::first_try(
            aws_sdk_dynamodb::config::Region::new(config.region.clone()),
        );

        let sdk_cfg = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;

        let mut builder = DynConfigBuilder::from(&sdk_cfg);
        if let Some(url) = &config.endpoint_url {
            builder = builder.endpoint_url(url);
        }

        let inner = Client::from_conf(builder.build());

        Ok(Self {
            inner,
            db_hash,
            pool_id: config.pool_id,
        })
    }

    /// Access the underlying `aws_sdk_dynamodb::Client`.
    pub fn raw_client(&self) -> &Client {
        &self.inner
    }

    /// Return the cached `db_hash`.
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

/// Emit a `db_log!` entry for any DynamoDB operation.
pub(crate) fn emit_db_log(
    db_hash: u32,
    table: &str,
    op_type: u8,
    duration_us: u32,
    rows_affected: u32,
    error_code: u8,
    pool_id: u16,
) {
    let table_hash = register_str(table);
    db_log!(
        Info,
        DbPayload {
            db_hash,
            table_hash,
            duration_us,
            rows_affected,
            op_type,
            error_code,
            pool_id,
            ..DbPayload::default()
        }
    );
}

// =============================================================================
// Internal helper — stable numeric code from a SdkError
// =============================================================================

pub(crate) fn fault_code_from_sdk_err<E: std::fmt::Debug>(e: &E) -> u32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    format!("{:?}", e).hash(&mut h);
    (h.finish() & 0xFFFF_FFFF) as u32
}
