// =============================================================================
// vil_db_timeseries::config — TimeseriesConfig
// =============================================================================
//
// Configuration for the time-series client.
// Config structs use External layout profile (setup-time data).
// =============================================================================

use serde::{Deserialize, Serialize};

/// Configuration for the time-series client wrapper.
///
/// # Example (YAML — InfluxDB)
/// ```yaml
/// host: "http://localhost:8086"
/// org: "myorg"
/// token: "my-token"
/// bucket: "metrics"
/// pool_id: 0
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeseriesConfig {
    /// InfluxDB host URL (e.g. `"http://localhost:8086"`).
    pub host: String,
    /// InfluxDB organization name.
    pub org: String,
    /// InfluxDB API token.
    pub token: String,
    /// Default bucket/measurement namespace.
    pub bucket: String,
    /// Logical pool/shard ID — stored in `DbPayload.pool_id`.
    pub pool_id: u16,
}

impl TimeseriesConfig {
    /// Create a minimal config for InfluxDB.
    pub fn new(
        host: impl Into<String>,
        org: impl Into<String>,
        token: impl Into<String>,
        bucket: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            org: org.into(),
            token: token.into(),
            bucket: bucket.into(),
            pool_id: 0,
        }
    }
}
