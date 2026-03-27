// =============================================================================
// Sidecar Configuration — YAML-driven sidecar definitions
// =============================================================================

use crate::reconnect::ReconnectPolicy;
use serde::{Deserialize, Serialize};

/// Configuration for a single sidecar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarConfig {
    /// Sidecar name (used for registry lookup and SHM naming).
    pub name: String,

    /// Optional command to auto-spawn the sidecar process.
    /// If None, sidecar must self-register via UDS.
    #[serde(default)]
    pub command: Option<String>,

    /// Unix domain socket path. Defaults to `/tmp/vil_sidecar_{name}.sock`.
    #[serde(default)]
    pub socket: Option<String>,

    /// SHM region size in bytes. Default: 64 MB.
    #[serde(default = "default_shm_size")]
    pub shm_size: u64,

    /// Health check interval in milliseconds. Default: 5000.
    #[serde(default = "default_health_interval")]
    pub health_interval_ms: u64,

    /// Invocation timeout in milliseconds. Default: 30000.
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    /// Number of retries on invocation failure. Default: 0.
    #[serde(default)]
    pub retry: u8,

    /// Number of concurrent connections to the sidecar. Default: 4.
    #[serde(default = "default_pool_size")]
    pub pool_size: usize,

    /// Maximum number of in-flight requests (0 = unlimited). Default: 1000.
    #[serde(default = "default_max_in_flight")]
    pub max_in_flight: u64,

    /// Optional authentication token for handshake.
    #[serde(default)]
    pub auth_token: Option<String>,

    /// Failover configuration.
    #[serde(default)]
    pub failover: Option<FailoverConfig>,

    /// Reconnect policy for dropped connections.
    #[serde(default = "default_reconnect_policy")]
    pub reconnect_policy: ReconnectPolicy,
}

impl SidecarConfig {
    /// Create a minimal config with just a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            command: None,
            socket: None,
            shm_size: default_shm_size(),
            health_interval_ms: default_health_interval(),
            timeout_ms: default_timeout(),
            retry: 0,
            pool_size: default_pool_size(),
            max_in_flight: default_max_in_flight(),
            auth_token: None,
            failover: None,
            reconnect_policy: default_reconnect_policy(),
        }
    }

    /// Set the spawn command.
    pub fn command(mut self, cmd: impl Into<String>) -> Self {
        self.command = Some(cmd.into());
        self
    }

    /// Set the timeout in milliseconds.
    pub fn timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Set the SHM size.
    pub fn shm_size(mut self, bytes: u64) -> Self {
        self.shm_size = bytes;
        self
    }

    /// Set the connection pool size.
    pub fn pool_size(mut self, n: usize) -> Self {
        self.pool_size = n;
        self
    }

    /// Set the maximum number of in-flight requests (0 = unlimited).
    pub fn max_in_flight(mut self, n: u64) -> Self {
        self.max_in_flight = n;
        self
    }

    /// Set the maximum reconnect retries.
    pub fn reconnect_max_retries(mut self, n: u32) -> Self {
        self.reconnect_policy.max_retries = n;
        self
    }

    /// Set the reconnect backoff parameters (base and max in milliseconds).
    pub fn reconnect_backoff_ms(mut self, base: u64, max: u64) -> Self {
        self.reconnect_policy.base_backoff_ms = base;
        self.reconnect_policy.max_backoff_ms = max;
        self
    }

    /// Resolve the socket path (use explicit or generate from name).
    pub fn socket_path(&self) -> String {
        self.socket
            .clone()
            .unwrap_or_else(|| crate::transport::socket_path(&self.name))
    }

    /// Resolve the SHM path.
    pub fn shm_path(&self) -> String {
        crate::transport::shm_path(&self.name)
    }
}

/// Failover configuration for a sidecar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverConfig {
    /// Name of backup sidecar to failover to.
    #[serde(default)]
    pub backup: Option<String>,

    /// WASM module to fall back to if all sidecars are down.
    #[serde(default)]
    pub fallback_wasm: Option<String>,

    /// Circuit breaker failure threshold.
    #[serde(default = "default_cb_threshold")]
    pub failure_threshold: u32,

    /// Circuit breaker cooldown in seconds.
    #[serde(default = "default_cb_cooldown")]
    pub cooldown_secs: u32,
}

fn default_shm_size() -> u64 {
    64 * 1024 * 1024 // 64 MB
}
fn default_health_interval() -> u64 {
    5000
}
fn default_timeout() -> u64 {
    30000
}
fn default_pool_size() -> usize {
    4
}
fn default_max_in_flight() -> u64 {
    1000
}
fn default_reconnect_policy() -> ReconnectPolicy {
    ReconnectPolicy::default()
}
fn default_cb_threshold() -> u32 {
    5
}
fn default_cb_cooldown() -> u32 {
    30
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let cfg = SidecarConfig::new("fraud-checker");
        assert_eq!(cfg.name, "fraud-checker");
        assert_eq!(cfg.shm_size, 64 * 1024 * 1024);
        assert_eq!(cfg.timeout_ms, 30000);
        assert_eq!(cfg.pool_size, 4);
        assert_eq!(cfg.max_in_flight, 1000);
        assert_eq!(cfg.reconnect_policy.max_retries, 10);
        assert_eq!(cfg.reconnect_policy.base_backoff_ms, 100);
        assert_eq!(cfg.reconnect_policy.max_backoff_ms, 30000);
        assert!(cfg.command.is_none());
    }

    #[test]
    fn test_config_builder() {
        let cfg = SidecarConfig::new("ml-engine")
            .command("python -m ml_service")
            .timeout(60000)
            .shm_size(256 * 1024 * 1024);

        assert_eq!(cfg.command.as_deref(), Some("python -m ml_service"));
        assert_eq!(cfg.timeout_ms, 60000);
        assert_eq!(cfg.shm_size, 256 * 1024 * 1024);
    }

    #[test]
    fn test_pool_and_reconnect_builder() {
        let cfg = SidecarConfig::new("ml-engine")
            .pool_size(8)
            .max_in_flight(5000)
            .reconnect_max_retries(20)
            .reconnect_backoff_ms(200, 60000);

        assert_eq!(cfg.pool_size, 8);
        assert_eq!(cfg.max_in_flight, 5000);
        assert_eq!(cfg.reconnect_policy.max_retries, 20);
        assert_eq!(cfg.reconnect_policy.base_backoff_ms, 200);
        assert_eq!(cfg.reconnect_policy.max_backoff_ms, 60000);
    }

    #[test]
    fn test_socket_path_resolution() {
        let cfg = SidecarConfig::new("fraud");
        assert_eq!(cfg.socket_path(), "/tmp/vil_sidecar_fraud.sock");

        let cfg2 = SidecarConfig {
            socket: Some("/custom/path.sock".into()),
            ..SidecarConfig::new("fraud")
        };
        assert_eq!(cfg2.socket_path(), "/custom/path.sock");
    }

    #[test]
    fn test_yaml_deserialize() {
        let yaml = r#"
name: fraud-checker
command: "python -m fraud_service"
shm_size: 67108864
timeout_ms: 30000
retry: 3
pool_size: 4
failover:
  backup: fraud-checker-2
  fallback_wasm: fraud_basic.wasm
  failure_threshold: 5
  cooldown_secs: 30
"#;
        let cfg: SidecarConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.name, "fraud-checker");
        assert_eq!(cfg.retry, 3);
        assert_eq!(cfg.pool_size, 4);
        assert!(cfg.failover.is_some());
        let fo = cfg.failover.unwrap();
        assert_eq!(fo.backup.as_deref(), Some("fraud-checker-2"));
        assert_eq!(fo.fallback_wasm.as_deref(), Some("fraud_basic.wasm"));
    }
}
