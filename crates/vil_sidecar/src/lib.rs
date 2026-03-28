// =============================================================================
// vil_sidecar — VIL Sidecar Protocol for External Process Integration
// =============================================================================
//
// Enables external processes (Python, Go, Java, etc.) to participate as
// VIL Process activities via zero-copy SHM IPC.
//
// Architecture:
//   - Transport: Unix Domain Socket (descriptors only, ~48 bytes per message)
//   - Data Plane: /dev/shm/vil_sc_{name} (zero-copy via mmap)
//   - Protocol: length-prefixed JSON (or MessagePack with `msgpack` feature)
//
// Usage (Host side — vil-server):
//   let registry = SidecarRegistry::new();
//   registry.register(SidecarConfig::new("fraud").command("python fraud.py"));
//   lifecycle::connect_sidecar(&registry, "fraud").await?;
//   let resp = dispatcher::invoke(&registry, "fraud", "check", &data).await?;
//
// Usage (Sidecar side — Python/Go/etc.):
//   let mut conn = SidecarConnection::connect(socket_path).await?;
//   conn.send(&Message::Handshake(handshake)).await?;

// Sprint 1: Protocol, Transport, SHM, Config
pub mod config;
pub mod protocol;
pub mod shm_bridge;
pub mod transport;

// Sprint 2: Registry, Lifecycle, Metrics, Dispatcher
pub mod dispatcher;
pub mod failover;
pub mod lifecycle;
pub mod metrics;
pub mod registry;

// Sprint 3: Connection Management
pub mod pool;
pub mod reconnect;

// ── Re-export: Protocol ──
pub use protocol::{
    decode_message, encode_message, fnv1a_hash, Handshake, HandshakeAck, HealthOk, Invoke,
    InvokeResult, InvokeStatus, Message, ShmDescriptor,
};

// ── Re-export: Transport ──
pub use transport::{shm_path, socket_path, SidecarConnection, SidecarListener, TransportError};

// ── Re-export: SHM ──
pub use shm_bridge::{remove_shm_region, ShmBridgeError, ShmRegion, DEFAULT_SHM_SIZE};

// ── Re-export: Config ──
pub use config::{FailoverConfig, SidecarConfig};

// ── Re-export: Registry ──
pub use registry::{SidecarEntry, SidecarHealth, SidecarRegistry};

// ── Re-export: Lifecycle ──
pub use lifecycle::{
    connect_sidecar, drain_sidecar, health_check, spawn_health_loop, LifecycleError,
};

// ── Re-export: Dispatcher ──
pub use dispatcher::{invoke, invoke_with_retry, DispatchError, InvokeResponse};

// ── Re-export: Failover ──
pub use failover::{invoke_with_failover, CircuitBreaker, FailoverError};

// ── Re-export: Metrics ──
pub use metrics::{MetricsSnapshot, SidecarMetrics};

// ── Re-export: Connection Pool ──
pub use pool::{ConnectionPool, PoolConfig, PoolError, PooledConnection};

// ── Re-export: Reconnect ──
pub use reconnect::{reconnect_with_backoff, ReconnectError, ReconnectPolicy};
