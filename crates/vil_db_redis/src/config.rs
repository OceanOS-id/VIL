use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    #[serde(default = "default_max")]
    pub max_connections: u32,
    #[serde(default = "default_db")]
    pub database: u32,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub services: Vec<String>,
}

fn default_max() -> u32 {
    20
}
fn default_db() -> u32 {
    0
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379".into(),
            max_connections: 20,
            database: 0,
            password: None,
            services: Vec::new(),
        }
    }
}

impl RedisConfig {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }
}
