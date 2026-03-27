// =============================================================================
// VIL Server Mesh — Tri-Lane Router
// =============================================================================
//
// Per-service-pair Tri-Lane channels:
//   - Trigger Lane: request init, auth tokens, session start
//   - Data Lane: payload stream (zero-copy SHM)
//   - Control Lane: backpressure signals, circuit breaker, health
//
// Key advantage: Control Lane is NEVER blocked by Data Lane congestion.
// Spring/Quarkus multiplex everything in one channel — control signals
// can be delayed when data is overloaded. VIL separates them physically.

use std::sync::Arc;

use dashmap::DashMap;
use vil_shm::ExchangeHeap;

use crate::shm_bridge::{ShmMeshChannel, ShmMeshReceiver, ShmChannelError};
use crate::{Lane, MeshConfig};

/// A set of three separated channels for one service pair.
///
/// Each lane has its own channel — congestion on Data Lane
/// cannot block Control Lane signals.
pub struct TriLaneChannels {
    pub trigger: ShmMeshChannel,
    pub data: ShmMeshChannel,
    pub control: ShmMeshChannel,
}

/// Receiver side of a Tri-Lane channel set.
pub struct TriLaneReceivers {
    pub trigger: ShmMeshReceiver,
    pub data: ShmMeshReceiver,
    pub control: ShmMeshReceiver,
}

/// Route key for a service pair: "from→to"
fn route_key(from: &str, to: &str) -> String {
    format!("{}→{}", from, to)
}

/// The Tri-Lane Router manages per-service-pair SHM channels.
///
/// When service A needs to talk to service B, the router creates
/// three SHM-backed channels (Trigger/Data/Control) for that pair.
/// Each lane operates independently — no head-of-line blocking.
pub struct TriLaneRouter {
    heap: Arc<ExchangeHeap>,
    /// Map of "from→to" -> TriLaneChannels (sender side)
    senders: DashMap<String, Arc<TriLaneChannels>>,
    /// Channel buffer size
    buffer_size: usize,
    /// SHM region size per lane
    region_size: usize,
}

impl TriLaneRouter {
    pub fn new(heap: Arc<ExchangeHeap>) -> Self {
        Self {
            heap,
            senders: DashMap::new(),
            buffer_size: 1024,
            region_size: 4 * 1024 * 1024, // 4MB per lane
        }
    }

    /// Configure channel buffer size (default: 1024).
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// Configure SHM region size per lane (default: 4MB).
    pub fn with_region_size(mut self, size: usize) -> Self {
        self.region_size = size;
        self
    }

    /// Register a route between two services.
    /// Creates three SHM channels (Trigger/Data/Control) for the pair.
    /// Returns the receivers for the target service.
    pub fn register_route(&self, from: &str, to: &str) -> TriLaneReceivers {
        let key = route_key(from, to);

        let (trigger_tx, trigger_rx) = ShmMeshChannel::new(
            self.heap.clone(),
            &format!("{}_trigger_{}", from, to),
            self.buffer_size,
            self.region_size,
        );

        let (data_tx, data_rx) = ShmMeshChannel::new(
            self.heap.clone(),
            &format!("{}_data_{}", from, to),
            self.buffer_size,
            self.region_size,
        );

        let (control_tx, control_rx) = ShmMeshChannel::new(
            self.heap.clone(),
            &format!("{}_ctrl_{}", from, to),
            // Control lane: smaller buffer but higher priority
            std::cmp::max(self.buffer_size / 4, 64),
            // Control lane: smaller region (signals are tiny)
            std::cmp::max(self.region_size / 16, 64 * 1024),
        );

        let channels = Arc::new(TriLaneChannels {
            trigger: trigger_tx,
            data: data_tx,
            control: control_tx,
        });

        self.senders.insert(key.clone(), channels);

        tracing::info!(
            from = %from,
            to = %to,
            "tri-lane route registered (Trigger + Data + Control)"
        );

        TriLaneReceivers {
            trigger: trigger_rx,
            data: data_rx,
            control: control_rx,
        }
    }

    /// Send data on a specific lane between two services.
    pub async fn send(
        &self,
        from: &str,
        to: &str,
        lane: Lane,
        data: &[u8],
    ) -> Result<usize, ShmChannelError> {
        let key = route_key(from, to);
        let channels = self.senders.get(&key)
            .ok_or(ShmChannelError::ChannelClosed)?;

        match lane {
            Lane::Trigger => channels.trigger.send(from, to, lane, data).await,
            Lane::Data => channels.data.send(from, to, lane, data).await,
            Lane::Control => channels.control.send(from, to, lane, data).await,
        }
    }

    /// Apply a MeshConfig to register all defined routes.
    pub fn apply_config(&self, config: &MeshConfig) -> Vec<(String, TriLaneReceivers)> {
        let mut receivers = Vec::new();

        // Group routes by service pair
        let mut pairs = std::collections::HashSet::new();
        for route in &config.routes {
            pairs.insert((route.from.clone(), route.to.clone()));
        }

        for (from, to) in pairs {
            let rx = self.register_route(&from, &to);
            receivers.push((route_key(&from, &to), rx));
        }

        receivers
    }

    /// Get the number of registered route pairs.
    pub fn route_count(&self) -> usize {
        self.senders.len()
    }

    /// List all registered route keys.
    pub fn route_keys(&self) -> Vec<String> {
        self.senders.iter().map(|e| e.key().clone()).collect()
    }
}

// =============================================================================
// TCP Tri-Lane Transport (production-grade, cross-host)
// =============================================================================

pub use crate::tcp_transport::{TcpTriLaneRouter, TcpTriLaneSender, TcpLaneError};

// =============================================================================
// Unified Transport — auto-selects SHM or TCP
// =============================================================================

/// Transport that automatically selects SHM (co-located) or TCP (remote).
///
/// - **SHM**: Zero-copy via shared memory (~50ns, co-located only).
/// - **TCP**: Length-prefixed binary framing (~50-500us, cross-host).
pub enum Transport {
    /// Zero-copy via shared memory (~50ns, co-located only)
    Shm(Arc<TriLaneRouter>),
    /// TCP for remote services (~50-500us, cross-host)
    Tcp(Arc<TcpTriLaneRouter>),
}

impl Transport {
    /// Auto-detect: if the target service has a registered remote peer
    /// address in the TCP router, use TCP. Otherwise use SHM.
    pub fn auto(
        shm_router: Arc<TriLaneRouter>,
        tcp_router: Arc<TcpTriLaneRouter>,
        to_service: &str,
    ) -> Self {
        if tcp_router.has_peer(to_service) {
            Transport::Tcp(tcp_router)
        } else {
            Transport::Shm(shm_router)
        }
    }

    pub async fn send(
        &self,
        from: &str,
        to: &str,
        lane: Lane,
        data: &[u8],
    ) -> Result<usize, String> {
        match self {
            Transport::Shm(router) => {
                router.send(from, to, lane, data).await
                    .map_err(|e| e.to_string())
            }
            Transport::Tcp(router) => {
                router.send(from, to, lane, data).await
                    .map_err(|e| e.to_string())
            }
        }
    }
}
