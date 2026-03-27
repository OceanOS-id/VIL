// =============================================================================
// vil_db_mongo::config — MongoConfig
// =============================================================================
//
// Configuration for the MongoDB client.
// Uses External layout profile (setup-time data, heap types acceptable here).
// Complies with COMPLIANCE.md §4: config structs may use External profile.
// =============================================================================

use serde::{Deserialize, Serialize};

/// Configuration for the MongoDB client wrapper.
///
/// This crate spawns no internal threads beyond the mongodb driver pool.
/// Add the driver's connection pool threads to your `LogConfig.threads` count
/// for optimal log ring sizing.
///
/// # Example (YAML)
/// ```yaml
/// uri: "mongodb://localhost:27017"
/// database: "myapp"
/// min_pool: 2
/// max_pool: 16
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MongoConfig {
    /// MongoDB connection URI (e.g. `"mongodb://localhost:27017"`).
    pub uri: String,
    /// Target database name.
    pub database: String,
    /// Minimum connection pool size. Defaults to driver default when `None`.
    pub min_pool: Option<u32>,
    /// Maximum connection pool size. Defaults to driver default when `None`.
    pub max_pool: Option<u32>,
}

impl MongoConfig {
    /// Create a minimal config with only URI and database set.
    pub fn new(uri: impl Into<String>, database: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            database: database.into(),
            min_pool: None,
            max_pool: None,
        }
    }
}
