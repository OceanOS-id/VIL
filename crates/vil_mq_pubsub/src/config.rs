// =============================================================================
// vil_mq_pubsub::config — PubSubConfig
// =============================================================================

use serde::{Deserialize, Serialize};

/// Google Cloud Pub/Sub configuration.
///
/// Config types use External layout profile (setup-time only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubSubConfig {
    /// GCP project ID.
    pub project_id: String,
    /// Pub/Sub topic name (short name, not full resource path).
    pub topic: String,
    /// Subscription name for consuming.
    pub subscription: String,
    /// Optional emulator endpoint (e.g. "localhost:8085" for local testing).
    pub emulator_host: Option<String>,
    /// Max messages to pull per receive call.
    #[serde(default = "default_max_msgs")]
    pub max_messages: i32,
    /// Ack deadline in seconds.
    #[serde(default = "default_ack_deadline")]
    pub ack_deadline_secs: i32,
}

fn default_max_msgs() -> i32 { 10 }
fn default_ack_deadline() -> i32 { 60 }

impl PubSubConfig {
    pub fn new(project_id: &str, topic: &str, subscription: &str) -> Self {
        Self {
            project_id: project_id.into(),
            topic: topic.into(),
            subscription: subscription.into(),
            emulator_host: None,
            max_messages: default_max_msgs(),
            ack_deadline_secs: default_ack_deadline(),
        }
    }

    pub fn with_emulator(mut self, host: &str) -> Self {
        self.emulator_host = Some(host.into());
        self
    }

    /// Full topic resource path.
    pub fn topic_path(&self) -> String {
        format!("projects/{}/topics/{}", self.project_id, self.topic)
    }

    /// Full subscription resource path.
    pub fn subscription_path(&self) -> String {
        format!(
            "projects/{}/subscriptions/{}",
            self.project_id, self.subscription
        )
    }
}
