// =============================================================================
// vil_mq_pulsar::config — PulsarConfig
// =============================================================================

use serde::{Deserialize, Serialize};

/// Apache Pulsar connection configuration.
///
/// Config types use External layout profile (setup-time only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulsarConfig {
    /// Pulsar broker URL, e.g. "pulsar://localhost:6650".
    pub url: String,
    /// Pulsar tenant name.
    pub tenant: String,
    /// Pulsar namespace.
    pub namespace: String,
    /// Optional authentication token.
    pub auth_token: Option<String>,
    /// Operation timeout in milliseconds.
    #[serde(default = "default_timeout_ms")]
    pub operation_timeout_ms: u64,
    /// Connection timeout in milliseconds.
    #[serde(default = "default_conn_timeout_ms")]
    pub connection_timeout_ms: u64,
}

fn default_timeout_ms() -> u64 { 30_000 }
fn default_conn_timeout_ms() -> u64 { 5_000 }

impl PulsarConfig {
    pub fn new(url: &str, tenant: &str, namespace: &str) -> Self {
        Self {
            url: url.into(),
            tenant: tenant.into(),
            namespace: namespace.into(),
            auth_token: None,
            operation_timeout_ms: default_timeout_ms(),
            connection_timeout_ms: default_conn_timeout_ms(),
        }
    }

    pub fn with_token(mut self, token: &str) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    /// Build the fully-qualified topic name for a given topic.
    pub fn topic_fqn(&self, topic: &str) -> String {
        format!("persistent://{}/{}/{}", self.tenant, self.namespace, topic)
    }
}
