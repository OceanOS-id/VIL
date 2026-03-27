// =============================================================================
// vil_db_timeseries::client — TimeseriesClient
// =============================================================================
//
// Time-series client wrapper with VIL semantic log integration.
//
// Backends selected by Cargo feature:
//   - `influxdb`  — InfluxDB v2 via the `influxdb2` crate
//   - `timescale` — TimescaleDB via vil_db_sqlx (see notes below)
//
// No println!, tracing::info!, or any non-VIL log call.
// =============================================================================

use vil_log::{db_log, types::DbPayload};
use vil_log::dict::register_str;

use crate::config::TimeseriesConfig;
#[allow(unused_imports)]
use crate::error::TimeseriesFault;
#[allow(unused_imports)]
use crate::types::TimeseriesResult;

/// Time-series client wrapper with integrated VIL semantic logging.
///
/// Backend is selected at compile time via Cargo features:
/// - `influxdb`  — InfluxDB v2 (`influxdb2` crate)
/// - `timescale` — TimescaleDB (delegate to `vil_db_sqlx`)
///
/// Every operation emits a `db_log!` entry with:
/// - `db_hash`       — FxHash of the bucket/database name
/// - `query_hash`    — FxHash of the Flux query (influxdb) or SQL (timescale)
/// - `duration_us`   — Wall-clock time of the operation
/// - `rows_affected` — Points written / rows returned
/// - `op_type`       — 0=QUERY 1=WRITE
/// - `error_code`    — 0 on success, non-zero on fault
pub struct TimeseriesClient {
    /// Cached FxHash of the bucket name.
    db_hash: u32,
    /// Logical pool ID forwarded to DbPayload.
    pool_id: u16,
    /// InfluxDB v2 client (only when `influxdb` feature is enabled).
    #[cfg(feature = "influxdb")]
    influx: influxdb2::Client,
    /// Stored config for reference.
    config: TimeseriesConfig,
}

impl TimeseriesClient {
    /// Build a `TimeseriesClient`.
    ///
    /// With the `influxdb` feature, connects to InfluxDB v2.
    /// With the `timescale` feature, callers should use `vil_db_sqlx` directly
    /// and pass SQL through its interface; this client records a stub db_hash.
    pub async fn new(config: TimeseriesConfig) -> TimeseriesResult<Self> {
        let db_hash = register_str(&config.bucket);

        #[cfg(feature = "influxdb")]
        let influx = {
            influxdb2::Client::new(&config.host, &config.org, &config.token)
        };

        Ok(Self {
            db_hash,
            pool_id: config.pool_id,
            #[cfg(feature = "influxdb")]
            influx,
            config,
        })
    }

    /// Return the cached db_hash (FxHash of bucket name).
    pub fn db_hash(&self) -> u32 {
        self.db_hash
    }

    /// Return the pool_id.
    pub fn pool_id(&self) -> u16 {
        self.pool_id
    }

    /// Return a reference to the stored config.
    pub fn config(&self) -> &TimeseriesConfig {
        &self.config
    }

    /// Access the underlying InfluxDB client (only with `influxdb` feature).
    #[cfg(feature = "influxdb")]
    pub fn raw_influx(&self) -> &influxdb2::Client {
        &self.influx
    }
}

// =============================================================================
// Internal helper — emit a DbPayload log entry
// =============================================================================

/// Emit a `db_log!` entry for any time-series operation.
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
