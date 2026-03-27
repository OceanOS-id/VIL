// =============================================================================
// VIL Server Mesh — Tri-Lane SHM Service Mesh
// =============================================================================
//
// Provides zero-copy inter-service communication via shared memory for
// co-located services, with automatic TCP fallback for remote services.

pub mod channel;
pub mod discovery;
pub mod router;
pub mod shm_bridge;
pub mod tcp_transport;
pub mod tri_lane;
pub mod yaml_config;
pub mod shm_discovery;
pub mod backpressure;
pub mod mq_adapter;

// Sprint 14-15: Advanced Patterns
pub mod pipeline_dag;
pub mod scatter_gather;
pub mod dlq;
pub mod typed_rpc;

// Sprint 17-18: Events & Operations
pub mod event_bus;
pub mod cqrs;
pub mod load_balancer;

use serde::Deserialize;

/// Tri-Lane types for service mesh communication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Lane {
    /// Request initiation, auth tokens, session start
    Trigger,
    /// Payload stream, response body, file upload (zero-copy SHM)
    Data,
    /// Backpressure signals, circuit breaker, health propagation
    Control,
}

impl std::fmt::Display for Lane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Lane::Trigger => write!(f, "Trigger"),
            Lane::Data => write!(f, "Data"),
            Lane::Control => write!(f, "Control"),
        }
    }
}

/// Transfer mode for mesh routes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum MeshMode {
    /// Zero-copy write via SHM (for data payloads)
    LoanWrite,
    /// Direct read from SHM buffer
    LoanRead,
    /// Copy data (for small control messages)
    Copy,
}

/// A route definition in the service mesh.
#[derive(Debug, Clone, Deserialize)]
pub struct MeshRoute {
    /// Source service name
    pub from: String,
    /// Target service name
    pub to: String,
    /// Which lane(s) this route uses
    pub lane: Lane,
    /// Transfer mode
    pub mode: MeshMode,
}

/// Mesh configuration parsed from YAML.
#[derive(Debug, Clone, Deserialize)]
pub struct MeshConfig {
    /// Service mesh routes
    #[serde(default)]
    pub routes: Vec<MeshRoute>,
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self {
            routes: Vec::new(),
        }
    }
}

/// Builder for configuring the service mesh programmatically.
pub struct MeshBuilder {
    routes: Vec<MeshRoute>,
}

impl MeshBuilder {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
        }
    }

    /// Add a route between two services.
    pub fn route(
        mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        lane: Lane,
        mode: MeshMode,
    ) -> Self {
        self.routes.push(MeshRoute {
            from: from.into(),
            to: to.into(),
            lane,
            mode,
        });
        self
    }

    /// Build the mesh configuration.
    pub fn build(self) -> MeshConfig {
        MeshConfig {
            routes: self.routes,
        }
    }
}

impl Default for MeshBuilder {
    fn default() -> Self {
        Self::new()
    }
}
