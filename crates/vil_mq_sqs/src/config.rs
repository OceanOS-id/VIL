// =============================================================================
// vil_mq_sqs::config — SQS/SNS connection configuration
// =============================================================================

use serde::{Deserialize, Serialize};

/// AWS SQS configuration.
///
/// Config types use External layout profile (setup-time only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqsConfig {
    /// AWS region, e.g. "us-east-1".
    pub region: String,
    /// SQS queue URL.
    pub queue_url: String,
    /// Optional custom endpoint (for LocalStack / testing).
    pub endpoint: Option<String>,
    /// Max number of messages to receive per poll (1–10).
    #[serde(default = "default_max_msgs")]
    pub max_messages: i32,
    /// Visibility timeout in seconds.
    #[serde(default = "default_visibility")]
    pub visibility_timeout_secs: i32,
    /// Long-poll wait time in seconds (0 = short poll).
    #[serde(default = "default_wait")]
    pub wait_time_secs: i32,
}

fn default_max_msgs() -> i32 {
    10
}
fn default_visibility() -> i32 {
    30
}
fn default_wait() -> i32 {
    20
}

impl SqsConfig {
    pub fn new(region: &str, queue_url: &str) -> Self {
        Self {
            region: region.into(),
            queue_url: queue_url.into(),
            endpoint: None,
            max_messages: default_max_msgs(),
            visibility_timeout_secs: default_visibility(),
            wait_time_secs: default_wait(),
        }
    }

    pub fn with_endpoint(mut self, endpoint: &str) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub fn with_max_messages(mut self, n: i32) -> Self {
        self.max_messages = n.clamp(1, 10);
        self
    }
}
