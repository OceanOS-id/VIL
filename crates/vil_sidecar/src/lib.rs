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
pub mod protocol;
pub mod transport;
pub mod shm_bridge;
pub mod config;

// Sprint 2: Registry, Lifecycle, Metrics, Dispatcher
pub mod registry;
pub mod lifecycle;
pub mod metrics;
pub mod dispatcher;
pub mod failover;

// Sprint 3: Connection Management
pub mod pool;
pub mod reconnect;

// ── Re-export: Protocol ──
pub use protocol::{
    Message, Handshake, HandshakeAck, Invoke, InvokeResult, InvokeStatus,
    HealthOk, ShmDescriptor, fnv1a_hash,
    encode_message, decode_message,
};

// ── Re-export: Transport ──
pub use transport::{
    SidecarConnection, SidecarListener, TransportError,
    socket_path, shm_path,
};

// ── Re-export: SHM ──
pub use shm_bridge::{ShmRegion, ShmBridgeError, remove_shm_region, DEFAULT_SHM_SIZE};

// ── Re-export: Config ──
pub use config::{SidecarConfig, FailoverConfig};

// ── Re-export: Registry ──
pub use registry::{SidecarRegistry, SidecarEntry, SidecarHealth};

// ── Re-export: Lifecycle ──
pub use lifecycle::{
    connect_sidecar, health_check, drain_sidecar, spawn_health_loop,
    LifecycleError,
};

// ── Re-export: Dispatcher ──
pub use dispatcher::{invoke, invoke_with_retry, InvokeResponse, DispatchError};

// ── Re-export: Failover ──
pub use failover::{invoke_with_failover, CircuitBreaker, FailoverError};

// ── Re-export: Metrics ──
pub use metrics::{SidecarMetrics, MetricsSnapshot};

// ── Re-export: Connection Pool ──
pub use pool::{ConnectionPool, PoolConfig, PooledConnection, PoolError};

// ── Re-export: Reconnect ──
pub use reconnect::{ReconnectPolicy, reconnect_with_backoff, ReconnectError};
