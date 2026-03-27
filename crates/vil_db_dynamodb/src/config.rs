// =============================================================================
// vil_db_dynamodb::config — DynamoConfig
// =============================================================================
//
// Configuration for the DynamoDB client wrapper.
// Config structs use External layout profile (setup-time data).
// =============================================================================

use serde::{Deserialize, Serialize};

/// Configuration for the DynamoDB client wrapper.
///
/// # Example (YAML)
/// ```yaml
/// region: "us-east-1"
/// endpoint_url: null          # set to override (e.g. LocalStack)
/// pool_id: 0
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamoConfig {
    /// AWS region name (e.g. `"us-east-1"`).
    pub region: String,
    /// Optional endpoint URL override (e.g. `"http://localhost:4566"` for LocalStack).
    pub endpoint_url: Option<String>,
    /// Logical pool/shard ID — stored in `DbPayload.pool_id`.
    pub pool_id: u16,
}

impl DynamoConfig {
    /// Create a minimal config targeting the given region.
    pub fn new(region: impl Into<String>) -> Self {
        Self {
            region: region.into(),
            endpoint_url: None,
            pool_id: 0,
        }
    }

    /// Override the endpoint URL (useful for LocalStack / integration tests).
    pub fn with_endpoint(mut self, url: impl Into<String>) -> Self {
        self.endpoint_url = Some(url.into());
        self
    }
}
