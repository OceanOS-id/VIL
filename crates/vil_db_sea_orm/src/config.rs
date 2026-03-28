// =============================================================================
// VIL DB sea-orm — Configuration
// =============================================================================

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeaOrmConfig {
    #[serde(default = "default_driver")]
    pub driver: String,
    pub url: String,
    #[serde(default = "default_max")]
    pub max_connections: u32,
    #[serde(default = "default_min")]
    pub min_connections: u32,
    #[serde(default = "default_timeout")]
    pub connect_timeout_secs: u64,
    #[serde(default = "default_idle")]
    pub idle_timeout_secs: u64,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub services: Vec<String>,
}

fn default_driver() -> String {
    "sqlite".into()
}
fn default_max() -> u32 {
    10
}
fn default_min() -> u32 {
    1
}
fn default_timeout() -> u64 {
    5
}
fn default_idle() -> u64 {
    300
}

impl Default for SeaOrmConfig {
    fn default() -> Self {
        Self {
            driver: default_driver(),
            url: String::new(),
            max_connections: default_max(),
            min_connections: default_min(),
            connect_timeout_secs: default_timeout(),
            idle_timeout_secs: default_idle(),
            schema: None,
            services: Vec::new(),
        }
    }
}

impl SeaOrmConfig {
    pub fn postgres(url: &str) -> Self {
        Self {
            driver: "postgres".into(),
            url: url.into(),
            ..Default::default()
        }
    }
    pub fn mysql(url: &str) -> Self {
        Self {
            driver: "mysql".into(),
            url: url.into(),
            ..Default::default()
        }
    }
    pub fn sqlite(url: &str) -> Self {
        Self {
            driver: "sqlite".into(),
            url: url.into(),
            ..Default::default()
        }
    }
    pub fn max_connections(mut self, n: u32) -> Self {
        self.max_connections = n;
        self
    }
}
