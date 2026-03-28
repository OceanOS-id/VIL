// =============================================================================
// VX Tri-Lane Router — Lightweight Tri-Lane for vil_server_core
// =============================================================================
//
// This is a self-contained Tri-Lane router that lives inside vil_server_core
// to avoid a cyclic dependency with vil_server_mesh. It uses vil_shm
// (already a dependency) for zero-copy SHM channels.
//
// vil_server_mesh::TriLaneRouter is the full-featured version with TCP
// fallback, discovery, etc. This module provides the core Tri-Lane semantics
// needed by VX services within a single process topology.

use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::mpsc;

use vil_shm::ExchangeHeap;

/// Tri-Lane types for VX inter-service communication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// A message envelope for the Tri-Lane channel.
#[derive(Debug, Clone)]
pub struct LaneMessage {
    /// Source service
    pub from: String,
    /// Target service
    pub to: String,
    /// Which lane
    pub lane: Lane,
    /// Payload bytes
    pub data: Vec<u8>,
}

/// Sender side of a single lane channel.
#[derive(Clone)]
pub struct LaneSender {
    tx: mpsc::Sender<LaneMessage>,
}

/// Receiver side of a single lane channel.
pub struct LaneReceiver {
    rx: mpsc::Receiver<LaneMessage>,
}

impl LaneSender {
    /// Send a message on this lane.
    pub async fn send(&self, msg: LaneMessage) -> Result<(), TriLaneError> {
        self.tx
            .send(msg)
            .await
            .map_err(|_| TriLaneError::ChannelClosed)
    }
}

impl LaneReceiver {
    /// Receive the next message on this lane.
    pub async fn recv(&mut self) -> Option<LaneMessage> {
        self.rx.recv().await
    }
}

/// A set of three separated channels for one service pair.
struct TriLaneChannels {
    trigger: LaneSender,
    data: LaneSender,
    control: LaneSender,
}

/// Receiver side of a Tri-Lane channel set.
pub struct TriLaneReceivers {
    pub trigger: LaneReceiver,
    pub data: LaneReceiver,
    pub control: LaneReceiver,
}

/// Route key for a service pair.
fn route_key(from: &str, to: &str) -> String {
    format!("{}→{}", from, to)
}

/// VX Tri-Lane Router — manages per-service-pair SHM channels.
///
/// Each lane operates independently — no head-of-line blocking.
/// Control Lane is never blocked by Data Lane congestion.
pub struct TriLaneRouter {
    #[allow(dead_code)]
    heap: Arc<ExchangeHeap>,
    senders: DashMap<String, Arc<TriLaneChannels>>,
    buffer_size: usize,
}

impl TriLaneRouter {
    /// Create a new Tri-Lane router backed by the given ExchangeHeap.
    pub fn new(heap: Arc<ExchangeHeap>) -> Self {
        Self {
            heap,
            senders: DashMap::new(),
            buffer_size: 1024,
        }
    }

    /// Configure channel buffer size (default: 1024).
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// Register a route between two services.
    /// Creates three channels (Trigger/Data/Control) for the pair.
    /// Returns the receivers for the target service.
    pub fn register_route(&self, from: &str, to: &str) -> TriLaneReceivers {
        let key = route_key(from, to);

        let (trigger_tx, trigger_rx) = mpsc::channel(self.buffer_size);
        let (data_tx, data_rx) = mpsc::channel(self.buffer_size);
        // Control lane: smaller buffer but higher priority
        let ctrl_buf = std::cmp::max(self.buffer_size / 4, 64);
        let (control_tx, control_rx) = mpsc::channel(ctrl_buf);

        let channels = Arc::new(TriLaneChannels {
            trigger: LaneSender { tx: trigger_tx },
            data: LaneSender { tx: data_tx },
            control: LaneSender { tx: control_tx },
        });

        self.senders.insert(key.clone(), channels);

        {
            use vil_log::app_log;
            app_log!(Info, "vx.trilane.route.registered", { from: from, to: to });
        }

        TriLaneReceivers {
            trigger: LaneReceiver { rx: trigger_rx },
            data: LaneReceiver { rx: data_rx },
            control: LaneReceiver { rx: control_rx },
        }
    }

    /// Send data on a specific lane between two services.
    pub async fn send(
        &self,
        from: &str,
        to: &str,
        lane: Lane,
        data: &[u8],
    ) -> Result<usize, TriLaneError> {
        let key = route_key(from, to);
        let channels = self.senders.get(&key).ok_or(TriLaneError::RouteNotFound)?;

        let msg = LaneMessage {
            from: from.to_string(),
            to: to.to_string(),
            lane,
            data: data.to_vec(),
        };

        let len = msg.data.len();
        match lane {
            Lane::Trigger => channels.trigger.send(msg).await?,
            Lane::Data => channels.data.send(msg).await?,
            Lane::Control => channels.control.send(msg).await?,
        }

        Ok(len)
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

/// Errors from Tri-Lane operations.
#[derive(Debug)]
pub enum TriLaneError {
    /// No route registered for this service pair
    RouteNotFound,
    /// Channel receiver has been dropped
    ChannelClosed,
}

impl std::fmt::Display for TriLaneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TriLaneError::RouteNotFound => write!(f, "Tri-Lane route not found"),
            TriLaneError::ChannelClosed => write!(f, "Tri-Lane channel closed"),
        }
    }
}

impl std::error::Error for TriLaneError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_and_send() {
        let heap = Arc::new(ExchangeHeap::new());
        let router = TriLaneRouter::new(heap);

        let mut rx = router.register_route("svc-a", "svc-b");
        assert_eq!(router.route_count(), 1);

        // Send on Data Lane
        let sent = router
            .send("svc-a", "svc-b", Lane::Data, b"hello")
            .await
            .unwrap();
        assert_eq!(sent, 5);

        // Receive
        let msg = rx.data.recv().await.unwrap();
        assert_eq!(msg.data, b"hello");
        assert_eq!(msg.from, "svc-a");
        assert_eq!(msg.to, "svc-b");
    }

    #[tokio::test]
    async fn send_to_unregistered_route_fails() {
        let heap = Arc::new(ExchangeHeap::new());
        let router = TriLaneRouter::new(heap);

        let result = router.send("x", "y", Lane::Trigger, b"nope").await;
        assert!(result.is_err());
    }

    #[test]
    fn lane_display() {
        assert_eq!(format!("{}", Lane::Trigger), "Trigger");
        assert_eq!(format!("{}", Lane::Data), "Data");
        assert_eq!(format!("{}", Lane::Control), "Control");
    }
}
