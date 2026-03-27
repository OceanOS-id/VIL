// =============================================================================
// vil_trigger_cdc::config — CdcConfig
// =============================================================================
//
// Configuration for the PostgreSQL logical replication CDC trigger.
// String fields are only present at setup time — not on hot paths.
// =============================================================================

/// Configuration for the VIL CDC (Change Data Capture) trigger.
///
/// Connects to a PostgreSQL instance using logical replication with the
/// `pgoutput` output plugin.
///
/// # Example YAML
/// ```yaml
/// cdc:
///   conn_string: "host=localhost port=5432 dbname=mydb user=repl password=secret"
///   slot_name: "vil_cdc_slot"
///   publication: "vil_pub"
/// ```
#[derive(Debug, Clone)]
pub struct CdcConfig {
    /// PostgreSQL connection string (libpq format).
    pub conn_string: String,
    /// Logical replication slot name (must already exist on the server).
    pub slot_name: String,
    /// Publication name created with `CREATE PUBLICATION`.
    pub publication: String,
}

impl CdcConfig {
    /// Construct a new `CdcConfig`.
    pub fn new(
        conn_string: impl Into<String>,
        slot_name: impl Into<String>,
        publication: impl Into<String>,
    ) -> Self {
        Self {
            conn_string: conn_string.into(),
            slot_name: slot_name.into(),
            publication: publication.into(),
        }
    }
}
