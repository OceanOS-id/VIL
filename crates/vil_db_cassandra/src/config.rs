// =============================================================================
// vil_db_cassandra::config — CassandraConfig
// =============================================================================
//
// Configuration for the Cassandra/ScyllaDB session.
// Config structs use External layout profile (setup-time data).
// =============================================================================

use serde::{Deserialize, Serialize};

/// Configuration for the Cassandra/ScyllaDB session wrapper.
///
/// # Example (YAML)
/// ```yaml
/// contact_points:
///   - "127.0.0.1:9042"
/// keyspace: "myapp"
/// pool_id: 0
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CassandraConfig {
    /// One or more contact-point addresses (host:port).
    pub contact_points: Vec<String>,
    /// Default keyspace to use.
    pub keyspace: String,
    /// Logical pool/shard ID — stored in `DbPayload.pool_id`.
    pub pool_id: u16,
}

impl CassandraConfig {
    /// Create a minimal config with a single contact point.
    pub fn new(contact_point: impl Into<String>, keyspace: impl Into<String>) -> Self {
        Self {
            contact_points: vec![contact_point.into()],
            keyspace: keyspace.into(),
            pool_id: 0,
        }
    }
}
