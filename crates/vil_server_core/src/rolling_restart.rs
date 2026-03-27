// =============================================================================
// VIL Server — Graceful Rolling Restart Support
// =============================================================================
//
// Enables zero-downtime restarts by coordinating with load balancers
// and draining in-flight requests before shutdown.
//
// Lifecycle:
//   1. Receive restart signal (SIGUSR1 or POST /admin/restart)
//   2. Set readiness probe to "not ready" → LB stops routing new traffic
//   3. Wait for in-flight requests to drain (configurable timeout)
//   4. Trigger graceful shutdown
//   5. New instance starts, readiness probe → "ready" → LB routes traffic

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Rolling restart state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RestartPhase {
    /// Normal operation
    Running,
    /// Draining in-flight requests (not accepting new)
    Draining,
    /// All requests drained, shutting down
    ShuttingDown,
}

/// Rolling restart coordinator.
pub struct RestartCoordinator {
    phase: Arc<std::sync::RwLock<RestartPhase>>,
    draining: AtomicBool,
    in_flight: AtomicU64,
    drain_timeout: Duration,
}

impl RestartCoordinator {
    pub fn new(drain_timeout: Duration) -> Self {
        Self {
            phase: Arc::new(std::sync::RwLock::new(RestartPhase::Running)),
            draining: AtomicBool::new(false),
            in_flight: AtomicU64::new(0),
            drain_timeout,
        }
    }

    /// Start the drain phase (stop accepting new requests).
    pub fn start_drain(&self) {
        *self.phase.write().unwrap() = RestartPhase::Draining;
        self.draining.store(true, Ordering::Relaxed);
        tracing::info!("rolling restart: draining in-flight requests");
    }

    /// Check if the server is accepting new requests.
    pub fn is_accepting(&self) -> bool {
        !self.draining.load(Ordering::Relaxed)
    }

    /// Record a request entering.
    pub fn request_enter(&self) {
        self.in_flight.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a request completing.
    pub fn request_exit(&self) {
        let remaining = self.in_flight.fetch_sub(1, Ordering::Relaxed) - 1;
        if self.draining.load(Ordering::Relaxed) && remaining == 0 {
            *self.phase.write().unwrap() = RestartPhase::ShuttingDown;
            tracing::info!("rolling restart: all requests drained, shutting down");
        }
    }

    /// Wait for all in-flight requests to drain.
    pub async fn wait_for_drain(&self) -> bool {
        let start = std::time::Instant::now();
        loop {
            let remaining = self.in_flight.load(Ordering::Relaxed);
            if remaining == 0 {
                return true;
            }
            if start.elapsed() > self.drain_timeout {
                tracing::warn!(
                    remaining = remaining,
                    "rolling restart: drain timeout reached, forcing shutdown"
                );
                return false;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Get current in-flight count.
    pub fn in_flight(&self) -> u64 {
        self.in_flight.load(Ordering::Relaxed)
    }

    /// Get current phase.
    pub fn phase(&self) -> RestartPhase {
        *self.phase.read().unwrap()
    }

    /// Get status for admin endpoint.
    pub fn status(&self) -> RestartStatus {
        RestartStatus {
            phase: self.phase(),
            in_flight: self.in_flight(),
            accepting: self.is_accepting(),
            drain_timeout_secs: self.drain_timeout.as_secs(),
        }
    }
}

impl Default for RestartCoordinator {
    fn default() -> Self {
        Self::new(Duration::from_secs(30))
    }
}

#[derive(Debug, Serialize)]
pub struct RestartStatus {
    pub phase: RestartPhase,
    pub in_flight: u64,
    pub accepting: bool,
    pub drain_timeout_secs: u64,
}
