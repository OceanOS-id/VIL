use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    pub brokers: String,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default = "default_acks")]
    pub acks: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub security_protocol: Option<String>,
    #[serde(default)]
    pub sasl_mechanism: Option<String>,
    #[serde(default)]
    pub sasl_username: Option<String>,
    #[serde(default)]
    pub sasl_password: Option<String>,
}

fn default_acks() -> String { "all".into() }
fn default_timeout() -> u64 { 5000 }

impl KafkaConfig {
    pub fn new(brokers: &str) -> Self {
        Self {
            brokers: brokers.into(), group_id: None, topic: None,
            acks: "all".into(), timeout_ms: 5000,
            security_protocol: None, sasl_mechanism: None,
            sasl_username: None, sasl_password: None,
        }
    }

    pub fn group(mut self, id: &str) -> Self { self.group_id = Some(id.into()); self }
    pub fn topic(mut self, t: &str) -> Self { self.topic = Some(t.into()); self }
}
