// =============================================================================
// SidecarRegistry — Central registry of all connected sidecars
// =============================================================================
//
// Manages sidecar connections, health state, and SHM regions.
// Thread-safe via DashMap — multiple handlers can invoke sidecars concurrently.

use crate::config::SidecarConfig;
use crate::metrics::SidecarMetrics;
use crate::shm_bridge::ShmRegion;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::transport::SidecarConnection;

/// Health state of a sidecar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidecarHealth {
    /// Connected and responding to health checks.
    Healthy,
    /// Connected but health checks are failing.
    Unhealthy,
    /// Not connected (initial state or after disconnect).
    Disconnected,
    /// Draining (no new work, waiting for in-flight to complete).
    Draining,
    /// Shut down.
    Stopped,
}

impl std::fmt::Display for SidecarHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Unhealthy => write!(f, "unhealthy"),
            Self::Disconnected => write!(f, "disconnected"),
            Self::Draining => write!(f, "draining"),
            Self::Stopped => write!(f, "stopped"),
        }
    }
}

/// A registered sidecar entry with connection, config, health, and metrics.
pub struct SidecarEntry {
    /// Configuration for this sidecar.
    pub config: SidecarConfig,
    /// Active connection (Mutex for exclusive send/recv access).
    pub connection: Option<Arc<Mutex<SidecarConnection>>>,
    /// Shared memory region for this sidecar.
    pub shm: Option<Arc<ShmRegion>>,
    /// Current health state.
    pub health: SidecarHealth,
    /// Methods this sidecar supports (from handshake).
    pub methods: Vec<String>,
    /// Per-sidecar metrics.
    pub metrics: Arc<SidecarMetrics>,
    /// Process ID if auto-spawned.
    pub pid: Option<u32>,
}

impl SidecarEntry {
    /// Create a new entry from config (disconnected state).
    pub fn new(config: SidecarConfig) -> Self {
        Self {
            config,
            connection: None,
            shm: None,
            health: SidecarHealth::Disconnected,
            methods: Vec::new(),
            metrics: Arc::new(SidecarMetrics::new()),
            pid: None,
        }
    }

    /// Check if a method is supported by this sidecar.
    pub fn supports_method(&self, method: &str) -> bool {
        self.methods.iter().any(|m| m == method)
    }

    /// Check if the sidecar is available for invocation.
    pub fn is_available(&self) -> bool {
        self.health == SidecarHealth::Healthy && self.connection.is_some()
    }
}

/// Central registry of all sidecars (thread-safe).
pub struct SidecarRegistry {
    entries: DashMap<String, SidecarEntry>,
}

impl SidecarRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
        }
    }

    /// Register a sidecar config (does not connect — call lifecycle::connect after).
    pub fn register(&self, config: SidecarConfig) {
        let name = config.name.clone();
        tracing::info!(sidecar = %name, "registered sidecar");
        self.entries.insert(name, SidecarEntry::new(config));
    }

    /// Get a reference to a sidecar entry.
    pub fn get(&self, name: &str) -> Option<dashmap::mapref::one::Ref<'_, String, SidecarEntry>> {
        self.entries.get(name)
    }

    /// Get a mutable reference to a sidecar entry.
    pub fn get_mut(
        &self,
        name: &str,
    ) -> Option<dashmap::mapref::one::RefMut<'_, String, SidecarEntry>> {
        self.entries.get_mut(name)
    }

    /// Remove a sidecar from the registry.
    pub fn remove(&self, name: &str) -> Option<SidecarEntry> {
        self.entries.remove(name).map(|(_, v)| v)
    }

    /// List all sidecar names.
    pub fn names(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.key().clone()).collect()
    }

    /// Number of registered sidecars.
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// List sidecars with their health status.
    pub fn status_list(&self) -> Vec<(String, SidecarHealth)> {
        self.entries
            .iter()
            .map(|e| (e.key().clone(), e.value().health))
            .collect()
    }

    /// Get all healthy sidecar names.
    pub fn healthy_sidecars(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| e.value().health == SidecarHealth::Healthy)
            .map(|e| e.key().clone())
            .collect()
    }

    /// Find a sidecar that supports the given method.
    pub fn find_by_method(&self, method: &str) -> Option<String> {
        self.entries
            .iter()
            .find(|e| e.value().is_available() && e.value().supports_method(method))
            .map(|e| e.key().clone())
    }

    /// Aggregate Prometheus metrics for all sidecars.
    pub fn prometheus_metrics(&self) -> String {
        let mut output = String::new();
        for entry in self.entries.iter() {
            output.push_str(&entry.value().metrics.to_prometheus(entry.key()));
        }
        output
    }
}

impl Default for SidecarRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get() {
        let reg = SidecarRegistry::new();
        reg.register(SidecarConfig::new("fraud-checker"));
        reg.register(SidecarConfig::new("ml-engine"));

        assert_eq!(reg.count(), 2);
        assert!(reg.get("fraud-checker").is_some());
        assert!(reg.get("missing").is_none());
    }

    #[test]
    fn test_names_and_status() {
        let reg = SidecarRegistry::new();
        reg.register(SidecarConfig::new("svc-a"));
        reg.register(SidecarConfig::new("svc-b"));

        let names = reg.names();
        assert_eq!(names.len(), 2);

        let statuses = reg.status_list();
        assert!(statuses
            .iter()
            .all(|(_, h)| *h == SidecarHealth::Disconnected));
    }

    #[test]
    fn test_remove() {
        let reg = SidecarRegistry::new();
        reg.register(SidecarConfig::new("temp"));
        assert_eq!(reg.count(), 1);

        let removed = reg.remove("temp");
        assert!(removed.is_some());
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn test_find_by_method_disconnected() {
        let reg = SidecarRegistry::new();
        reg.register(SidecarConfig::new("fraud"));

        // Not available yet (disconnected, no connection)
        assert!(reg.find_by_method("fraud_check").is_none());
    }

    #[test]
    fn test_healthy_sidecars() {
        let reg = SidecarRegistry::new();
        reg.register(SidecarConfig::new("a"));
        reg.register(SidecarConfig::new("b"));

        // All disconnected initially
        assert_eq!(reg.healthy_sidecars().len(), 0);
    }

    #[test]
    fn test_prometheus_metrics() {
        let reg = SidecarRegistry::new();
        reg.register(SidecarConfig::new("fraud"));

        // Record some metrics
        {
            let entry = reg.get("fraud").unwrap();
            entry.metrics.invoke_start();
            entry.metrics.invoke_ok(100);
        }

        let prom = reg.prometheus_metrics();
        assert!(prom.contains("vil_sidecar_invocations_total{sidecar=\"fraud\"} 1"));
    }

    #[test]
    fn test_entry_supports_method() {
        let mut entry = SidecarEntry::new(SidecarConfig::new("test"));
        entry.methods = vec!["predict".into(), "score".into()];

        assert!(entry.supports_method("predict"));
        assert!(entry.supports_method("score"));
        assert!(!entry.supports_method("unknown"));
    }

    #[test]
    fn test_entry_is_available() {
        let entry = SidecarEntry::new(SidecarConfig::new("test"));
        assert!(!entry.is_available()); // Disconnected, no connection
    }
}
