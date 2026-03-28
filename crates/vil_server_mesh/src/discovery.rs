// =============================================================================
// VIL Server Mesh Discovery — Service discovery implementations
// =============================================================================

use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;

/// A service endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct Endpoint {
    /// Host address
    pub host: String,
    /// Port number
    pub port: u16,
    /// Whether this endpoint is healthy
    pub healthy: bool,
}

impl Endpoint {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            healthy: true,
        }
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Service registration info.
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub endpoints: Vec<Endpoint>,
    pub co_located: bool,
}

/// Health status of a service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Trait for service discovery implementations.
/// Users can implement this trait to integrate with Consul, etcd, etc.
#[async_trait]
pub trait ServiceDiscovery: Send + Sync + 'static {
    /// Resolve a service name to a list of endpoints.
    async fn resolve(&self, service_name: &str) -> Result<Vec<Endpoint>, DiscoveryError>;

    /// Register a service.
    async fn register(&self, service: ServiceInfo) -> Result<(), DiscoveryError>;

    /// Deregister a service.
    async fn deregister(&self, service_id: &str) -> Result<(), DiscoveryError>;

    /// Check health of a service.
    async fn health_check(&self, service_id: &str) -> Result<HealthStatus, DiscoveryError>;
}

/// Discovery error type.
#[derive(Debug)]
pub enum DiscoveryError {
    NotFound(String),
    ConnectionError(String),
    Other(String),
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryError::NotFound(s) => write!(f, "Service not found: {}", s),
            DiscoveryError::ConnectionError(s) => write!(f, "Connection error: {}", s),
            DiscoveryError::Other(s) => write!(f, "Discovery error: {}", s),
        }
    }
}

impl std::error::Error for DiscoveryError {}

// ============================================================================
// Tier 2: Config-based discovery (static from YAML/env)
// ============================================================================

/// Static service discovery from configuration.
pub struct ConfigDiscovery {
    services: HashMap<String, Vec<Endpoint>>,
}

impl ConfigDiscovery {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    /// Register a service endpoint from config.
    pub fn add_service(&mut self, name: impl Into<String>, endpoint: Endpoint) {
        self.services.entry(name.into()).or_default().push(endpoint);
    }
}

impl Default for ConfigDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ServiceDiscovery for ConfigDiscovery {
    async fn resolve(&self, service_name: &str) -> Result<Vec<Endpoint>, DiscoveryError> {
        self.services
            .get(service_name)
            .cloned()
            .ok_or_else(|| DiscoveryError::NotFound(service_name.to_string()))
    }

    async fn register(&self, _service: ServiceInfo) -> Result<(), DiscoveryError> {
        // Config discovery is static — registration is a no-op
        Ok(())
    }

    async fn deregister(&self, _service_id: &str) -> Result<(), DiscoveryError> {
        Ok(())
    }

    async fn health_check(&self, service_id: &str) -> Result<HealthStatus, DiscoveryError> {
        if self.services.contains_key(service_id) {
            Ok(HealthStatus::Healthy)
        } else {
            Ok(HealthStatus::Unknown)
        }
    }
}
