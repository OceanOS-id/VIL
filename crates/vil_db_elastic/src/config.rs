// =============================================================================
// vil_db_elastic::config — ElasticConfig
// =============================================================================

use serde::{Deserialize, Serialize};

/// Configuration for an Elasticsearch / OpenSearch client.
///
/// # Example
/// ```rust,ignore
/// let cfg = ElasticConfig {
///     url: "http://localhost:9200".into(),
///     username: Some("elastic".into()),
///     password: Some("changeme".into()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElasticConfig {
    /// Elasticsearch node URL, e.g. `"http://localhost:9200"`.
    pub url: String,

    /// Optional username for HTTP Basic auth.
    pub username: Option<String>,

    /// Optional password for HTTP Basic auth.
    pub password: Option<String>,
}

impl Default for ElasticConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:9200".into(),
            username: None,
            password: None,
        }
    }
}
