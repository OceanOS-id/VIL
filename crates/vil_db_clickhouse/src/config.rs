// =============================================================================
// vil_db_clickhouse::config — ClickHouseConfig
// =============================================================================
//
// Configuration for connecting to a ClickHouse server.
// Uses External layout (setup-time data, heap types acceptable here).
// =============================================================================

/// ClickHouse connection configuration.
///
/// All fields are setup-time only; they are never placed on a hot path.
/// Layout profile: **External** — heap types (`String`, `Option<String>`) are
/// acceptable because this struct is only used during `ChClient::new`.
///
/// # Example
/// ```rust,no_run
/// use vil_db_clickhouse::ClickHouseConfig;
///
/// let cfg = ClickHouseConfig {
///     url: "http://localhost:8123".to_string(),
///     database: "analytics".to_string(),
///     username: Some("default".to_string()),
///     password: None,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ClickHouseConfig {
    /// HTTP(S) URL of the ClickHouse server, e.g. `"http://localhost:8123"`.
    pub url: String,

    /// Database (schema) to use for all queries and inserts.
    pub database: String,

    /// Optional username for HTTP Basic auth.
    pub username: Option<String>,

    /// Optional password for HTTP Basic auth.
    pub password: Option<String>,
}

impl Default for ClickHouseConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8123".to_string(),
            database: "default".to_string(),
            username: None,
            password: None,
        }
    }
}
