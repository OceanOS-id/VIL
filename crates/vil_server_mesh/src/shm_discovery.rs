// =============================================================================
// VIL Server Mesh — SHM Discovery (Tier 1)
// =============================================================================
//
// Auto-discovers co-located services via vil_registry.
// When multiple services run in the same binary (unified mode),
// they are automatically discovered through the VIL runtime's
// process registry — no configuration needed.
//
// This is Tier 1 of the 3-tier discovery model:
//   Tier 1: SHM auto-discovery (this module) — zero config, co-located only
//   Tier 2: Config-based discovery (discovery.rs) — static YAML/env
//   Tier 3: Pluggable trait (discovery.rs) — Consul/etcd adapters

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use vil_rt::VastarRuntimeWorld;

use crate::discovery::{DiscoveryError, Endpoint, HealthStatus, ServiceDiscovery, ServiceInfo};

/// Automatic SHM-based service discovery for co-located services.
///
/// Queries VastarRuntimeWorld's process registry to find services
/// that are running in the same binary. These services communicate
/// via SHM (zero-copy, 1-5µs latency).
pub struct ShmDiscovery {
    runtime: Arc<VastarRuntimeWorld>,
    /// Map of service_name → local port (if exposed)
    local_services: HashMap<String, u16>,
}

impl ShmDiscovery {
    pub fn new(runtime: Arc<VastarRuntimeWorld>) -> Self {
        Self {
            runtime,
            local_services: HashMap::new(),
        }
    }

    /// Register a service as locally available.
    pub fn register_local(&mut self, name: impl Into<String>, port: u16) {
        self.local_services.insert(name.into(), port);
    }

    /// Check if a service is co-located (same binary).
    pub fn is_co_located(&self, service_name: &str) -> bool {
        self.local_services.contains_key(service_name)
    }

    /// Get all co-located service names.
    pub fn co_located_services(&self) -> Vec<String> {
        self.local_services.keys().cloned().collect()
    }

    /// Get process count from the runtime registry.
    pub fn process_count(&self) -> usize {
        self.runtime.registry_processes().len()
    }
}

#[async_trait]
impl ServiceDiscovery for ShmDiscovery {
    async fn resolve(&self, service_name: &str) -> Result<Vec<Endpoint>, DiscoveryError> {
        // First check local services
        if let Some(&port) = self.local_services.get(service_name) {
            return Ok(vec![Endpoint::new("127.0.0.1", port)]);
        }

        // Check runtime process registry
        let processes = self.runtime.registry_processes();
        for proc in &processes {
            let proc_name = format!("{:?}", proc.id);
            if proc_name.contains(service_name) {
                // Found in same runtime — resolve as localhost
                return Ok(vec![Endpoint::new("shm://local", 0)]);
            }
        }

        Err(DiscoveryError::NotFound(service_name.to_string()))
    }

    async fn register(&self, service: ServiceInfo) -> Result<(), DiscoveryError> {
        {
            use vil_log::app_log;
            app_log!(Info, "mesh.discovery.registered", { service: service.name.as_str() });
        }
        Ok(())
    }

    async fn deregister(&self, service_id: &str) -> Result<(), DiscoveryError> {
        {
            use vil_log::app_log;
            app_log!(Info, "mesh.discovery.deregistered", { service: service_id });
        }
        Ok(())
    }

    async fn health_check(&self, service_id: &str) -> Result<HealthStatus, DiscoveryError> {
        if self.local_services.contains_key(service_id) {
            // Co-located services are always healthy if the runtime is running
            Ok(HealthStatus::Healthy)
        } else {
            Ok(HealthStatus::Unknown)
        }
    }
}
