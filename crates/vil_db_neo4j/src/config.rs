// =============================================================================
// vil_db_neo4j::config — Neo4jConfig
// =============================================================================
//
// Configuration for the Neo4j graph client.
// Config structs use External layout profile (setup-time data).
// =============================================================================

use serde::{Deserialize, Serialize};

/// Configuration for the Neo4j graph client wrapper.
///
/// # Example (YAML)
/// ```yaml
/// uri: "bolt://localhost:7687"
/// user: "neo4j"
/// password: "password"
/// pool_id: 0
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neo4jConfig {
    /// Bolt/Neo4j URI (e.g. `"bolt://localhost:7687"`).
    pub uri: String,
    /// Username for authentication.
    pub user: String,
    /// Password for authentication.
    pub password: String,
    /// Logical pool/shard ID — stored in `DbPayload.pool_id`.
    pub pool_id: u16,
}

impl Neo4jConfig {
    /// Create a minimal config.
    pub fn new(
        uri: impl Into<String>,
        user: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            uri: uri.into(),
            user: user.into(),
            password: password.into(),
            pool_id: 0,
        }
    }
}
