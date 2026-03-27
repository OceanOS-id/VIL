use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsConfig {
    pub url: String,
    #[serde(default)]
    pub credentials: Option<NatsCredentials>,
    #[serde(default)]
    pub tls: bool,
    #[serde(default = "default_name")]
    pub client_name: String,
    #[serde(default = "default_reconnect")]
    pub max_reconnects: u32,
    #[serde(default = "default_buffer")]
    pub buffer_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsCredentials {
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
    pub nkey: Option<String>,
}

fn default_name() -> String { "vil-client".into() }
fn default_reconnect() -> u32 { 60 }
fn default_buffer() -> usize { 65536 }

impl NatsConfig {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.into(), credentials: None, tls: false,
            client_name: default_name(), max_reconnects: 60, buffer_size: 65536,
        }
    }

    pub fn with_token(mut self, token: &str) -> Self {
        self.credentials = Some(NatsCredentials {
            token: Some(token.into()), username: None, password: None, nkey: None,
        });
        self
    }

    pub fn with_userpass(mut self, user: &str, pass: &str) -> Self {
        self.credentials = Some(NatsCredentials {
            username: Some(user.into()), password: Some(pass.into()), token: None, nkey: None,
        });
        self
    }

    pub fn tls(mut self, enabled: bool) -> Self { self.tls = enabled; self }
    pub fn name(mut self, n: &str) -> Self { self.client_name = n.into(); self }
}
