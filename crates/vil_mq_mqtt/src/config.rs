use serde::{Deserialize, Serialize};

/// MQTT Quality of Service levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QoS {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

impl Default for QoS {
    fn default() -> Self {
        Self::AtLeastOnce
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    pub broker_url: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub qos: QoS,
    #[serde(default)]
    pub tls: bool,
    #[serde(default = "default_keepalive")]
    pub keepalive_secs: u64,
}

fn default_port() -> u16 {
    1883
}
fn default_keepalive() -> u64 {
    60
}

impl MqttConfig {
    pub fn new(broker_url: &str) -> Self {
        Self {
            broker_url: broker_url.into(),
            port: 1883,
            client_id: None,
            username: None,
            password: None,
            qos: QoS::AtLeastOnce,
            tls: false,
            keepalive_secs: 60,
        }
    }

    pub fn client_id(mut self, id: &str) -> Self {
        self.client_id = Some(id.into());
        self
    }
    pub fn qos(mut self, qos: QoS) -> Self {
        self.qos = qos;
        self
    }
    pub fn tls(mut self, enabled: bool) -> Self {
        self.tls = enabled;
        self
    }
}
