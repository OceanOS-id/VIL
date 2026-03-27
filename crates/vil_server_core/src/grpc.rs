// =============================================================================
// VIL Server gRPC — Tonic integration with Tri-Lane routing
// =============================================================================
//
// Provides gRPC server and client support for vil-server.
// Co-located services use SHM Tri-Lane, remote services use gRPC.
//
// Architecture:
//   - GrpcServer wraps tonic, serving alongside the Axum HTTP server
//   - GrpcClient auto-selects transport: SHM for co-located, gRPC for remote
//   - Both integrate with VilMetrics for auto-observability
//
// Note: Actual tonic integration requires protobuf service definitions.
// This module provides the infrastructure layer — users define their own
// .proto services and plug them into vil-server.

use std::net::SocketAddr;

/// Configuration for the gRPC server.
#[derive(Debug, Clone)]
pub struct GrpcConfig {
    /// gRPC listen port (separate from HTTP)
    pub port: u16,
    /// Maximum message size in bytes (default: 4MB)
    pub max_message_size: usize,
    /// Enable gRPC reflection (for grpcurl/grpcui)
    pub reflection: bool,
    /// Enable gRPC health check service
    pub health_check: bool,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            port: 50051,
            max_message_size: 4 * 1024 * 1024,
            reflection: true,
            health_check: true,
        }
    }
}

impl GrpcConfig {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            ..Default::default()
        }
    }

    pub fn addr(&self) -> SocketAddr {
        SocketAddr::from(([0, 0, 0, 0], self.port))
    }
}

/// gRPC transport selection for inter-service communication.
///
/// When two services need to communicate:
/// - Co-located (same binary): use SHM via Tri-Lane (1-5µs)
/// - Remote (different host): use gRPC (standard protobuf over HTTP/2)
///
/// The selection is automatic based on ShmDiscovery results.
#[derive(Debug, Clone)]
pub enum GrpcTransport {
    /// SHM via Tri-Lane — zero-copy, co-located only
    Shm {
        service_name: String,
    },
    /// Standard gRPC over HTTP/2 — for remote services
    Remote {
        endpoint: String,
    },
}

impl GrpcTransport {
    /// Create SHM transport for a co-located service.
    pub fn shm(service_name: impl Into<String>) -> Self {
        Self::Shm {
            service_name: service_name.into(),
        }
    }

    /// Create remote gRPC transport.
    pub fn remote(endpoint: impl Into<String>) -> Self {
        Self::Remote {
            endpoint: endpoint.into(),
        }
    }

    /// Check if this transport is SHM (co-located).
    pub fn is_shm(&self) -> bool {
        matches!(self, Self::Shm { .. })
    }

    /// Get the endpoint string.
    pub fn endpoint(&self) -> String {
        match self {
            Self::Shm { service_name } => format!("shm://{}", service_name),
            Self::Remote { endpoint } => endpoint.clone(),
        }
    }
}

/// gRPC service registry — tracks available gRPC services
/// and their transport mode (SHM or remote).
pub struct GrpcServiceRegistry {
    services: dashmap::DashMap<String, GrpcTransport>,
}

impl GrpcServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: dashmap::DashMap::new(),
        }
    }

    /// Register a gRPC service with its transport.
    pub fn register(&self, name: impl Into<String>, transport: GrpcTransport) {
        let name = name.into();
        tracing::info!(
            service = %name,
            transport = %transport.endpoint(),
            "gRPC service registered"
        );
        self.services.insert(name, transport);
    }

    /// Get the transport for a service.
    pub fn get_transport(&self, name: &str) -> Option<GrpcTransport> {
        self.services.get(name).map(|v| v.value().clone())
    }

    /// List all registered services.
    pub fn list_services(&self) -> Vec<(String, String)> {
        self.services.iter()
            .map(|e| (e.key().clone(), e.value().endpoint()))
            .collect()
    }

    /// Get count of registered services.
    pub fn service_count(&self) -> usize {
        self.services.len()
    }
}

impl Default for GrpcServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
