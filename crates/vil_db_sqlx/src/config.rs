// =============================================================================
// VIL DB sqlx — Configuration
// =============================================================================

use serde::{Deserialize, Serialize};

/// sqlx pool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlxConfig {
    /// Database driver: postgres, mysql, sqlite
    #[serde(default = "default_driver")]
    pub driver: String,
    /// Connection URL (may be encrypted: ENC[AES256:...])
    pub url: String,
    /// Maximum connections in the pool
    #[serde(default = "default_max_conn")]
    pub max_connections: u32,
    /// Minimum idle connections
    #[serde(default = "default_min_conn")]
    pub min_connections: u32,
    /// Connect timeout in seconds
    #[serde(default = "default_timeout")]
    pub connect_timeout_secs: u64,
    /// Idle connection timeout in seconds
    #[serde(default = "default_idle")]
    pub idle_timeout_secs: u64,
    /// SSL mode: disable, prefer, require
    #[serde(default = "default_ssl")]
    pub ssl_mode: String,
    /// Assigned services (empty = all services)
    #[serde(default)]
    pub services: Vec<String>,
}

fn default_driver() -> String { "sqlite".to_string() }
fn default_max_conn() -> u32 { 10 }
fn default_min_conn() -> u32 { 1 }
fn default_timeout() -> u64 { 5 }
fn default_idle() -> u64 { 300 }
fn default_ssl() -> String { "prefer".to_string() }

impl Default for SqlxConfig {
    fn default() -> Self {
        Self {
            driver: default_driver(),
            url: String::new(),
            max_connections: default_max_conn(),
            min_connections: default_min_conn(),
            connect_timeout_secs: default_timeout(),
            idle_timeout_secs: default_idle(),
            ssl_mode: default_ssl(),
            services: Vec::new(),
        }
    }
}

impl SqlxConfig {
    pub fn postgres(url: &str) -> Self {
        Self { driver: "postgres".into(), url: url.into(), ..Default::default() }
    }

    pub fn mysql(url: &str) -> Self {
        Self { driver: "mysql".into(), url: url.into(), ..Default::default() }
    }

    pub fn sqlite(url: &str) -> Self {
        Self { driver: "sqlite".into(), url: url.into(), ..Default::default() }
    }

    pub fn max_connections(mut self, n: u32) -> Self { self.max_connections = n; self }
    pub fn min_connections(mut self, n: u32) -> Self { self.min_connections = n; self }
    pub fn timeout(mut self, secs: u64) -> Self { self.connect_timeout_secs = secs; self }

    /// Check if this pool is assigned to a specific service.
    pub fn is_for_service(&self, service: &str) -> bool {
        self.services.is_empty()
            || self.services.contains(&"*".to_string())
            || self.services.contains(&service.to_string())
    }
}
