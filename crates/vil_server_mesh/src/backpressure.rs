// =============================================================================
// VIL Server Mesh — Control Lane Backpressure
// =============================================================================
//
// When a service is overloaded, it sends a backpressure signal through the
// Control Lane. Upstream services automatically throttle their request rate.
//
// This is a key differentiator: Spring/Quarkus have no built-in inter-service
// backpressure. vil-server's Control Lane ensures overloaded services are
// never flooded — and the signal path is physically separate from data.
//
// Signals:
//   Throttle(rate)  → reduce sending rate to N msg/sec
//   Pause           → stop sending until Resume
//   Resume          → resume normal sending
//   Drain           → graceful shutdown, finish in-flight then stop
//   HealthDegraded  → service is degraded, consider failover

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Backpressure signal sent through the Control Lane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackpressureSignal {
    /// Reduce sending rate to N messages per second
    Throttle { max_rate: u64 },
    /// Stop sending until Resume is received
    Pause,
    /// Resume normal sending after Pause
    Resume,
    /// Graceful drain — finish in-flight, then stop
    Drain,
    /// Service health is degraded
    HealthDegraded { reason: String },
    /// Service health restored
    HealthRestored,
}

impl BackpressureSignal {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }
}

/// Per-service backpressure state.
///
/// Each service tracks its own load and sends signals when thresholds
/// are exceeded. Upstream services monitor these signals and adjust.
pub struct BackpressureController {
    /// Service name
    service: String,
    /// Whether the service is currently paused
    paused: AtomicBool,
    /// Current throttle rate (0 = unlimited)
    throttle_rate: AtomicU64,
    /// Current in-flight request count
    in_flight: AtomicU64,
    /// Maximum in-flight before triggering backpressure
    max_in_flight: u64,
    /// High watermark (trigger throttle)
    high_watermark: u64,
    /// Low watermark (release throttle)
    low_watermark: u64,
}

impl BackpressureController {
    /// Create a new backpressure controller.
    ///
    /// - `max_in_flight`: hard limit — triggers Pause
    /// - `high_watermark`: soft limit — triggers Throttle
    /// - `low_watermark`: release point — triggers Resume
    pub fn new(service: &str, max_in_flight: u64) -> Self {
        Self {
            service: service.to_string(),
            paused: AtomicBool::new(false),
            throttle_rate: AtomicU64::new(0),
            in_flight: AtomicU64::new(0),
            max_in_flight,
            high_watermark: max_in_flight * 80 / 100, // 80%
            low_watermark: max_in_flight * 50 / 100,  // 50%
        }
    }

    /// Record a request entering the service.
    /// Returns a backpressure signal if thresholds are exceeded.
    pub fn request_enter(&self) -> Option<BackpressureSignal> {
        let current = self.in_flight.fetch_add(1, Ordering::Relaxed) + 1;

        if current >= self.max_in_flight {
            self.paused.store(true, Ordering::Relaxed);
            tracing::warn!(
                service = %self.service,
                in_flight = current,
                max = self.max_in_flight,
                "backpressure: PAUSE (max in-flight reached)"
            );
            return Some(BackpressureSignal::Pause);
        }

        if current >= self.high_watermark && !self.paused.load(Ordering::Relaxed) {
            let rate = self.max_in_flight.saturating_sub(current) * 100;
            self.throttle_rate.store(rate, Ordering::Relaxed);
            tracing::info!(
                service = %self.service,
                in_flight = current,
                throttle_rate = rate,
                "backpressure: THROTTLE"
            );
            return Some(BackpressureSignal::Throttle { max_rate: rate });
        }

        None
    }

    /// Record a request leaving the service.
    /// Returns a Resume signal if load drops below low watermark.
    pub fn request_exit(&self) -> Option<BackpressureSignal> {
        let current = self.in_flight.fetch_sub(1, Ordering::Relaxed) - 1;

        if self.paused.load(Ordering::Relaxed) && current <= self.low_watermark {
            self.paused.store(false, Ordering::Relaxed);
            self.throttle_rate.store(0, Ordering::Relaxed);
            tracing::info!(
                service = %self.service,
                in_flight = current,
                "backpressure: RESUME"
            );
            return Some(BackpressureSignal::Resume);
        }

        None
    }

    /// Check if the service is currently accepting requests.
    pub fn is_accepting(&self) -> bool {
        !self.paused.load(Ordering::Relaxed)
    }

    /// Get current in-flight count.
    pub fn in_flight(&self) -> u64 {
        self.in_flight.load(Ordering::Relaxed)
    }

    /// Get current throttle rate (0 = unlimited).
    pub fn throttle_rate(&self) -> u64 {
        self.throttle_rate.load(Ordering::Relaxed)
    }

    /// Get the service name.
    pub fn service(&self) -> &str {
        &self.service
    }
}

/// Upstream throttle responder.
///
/// Installed on the sending side — listens for backpressure signals
/// from the receiving service and adjusts sending behavior.
pub struct UpstreamThrottle {
    /// Whether upstream is currently paused
    paused: AtomicBool,
    /// Current allowed rate (0 = unlimited)
    allowed_rate: AtomicU64,
    /// Target service name
    target: String,
}

impl UpstreamThrottle {
    pub fn new(target: &str) -> Self {
        Self {
            paused: AtomicBool::new(false),
            allowed_rate: AtomicU64::new(0),
            target: target.to_string(),
        }
    }

    /// Process a backpressure signal from the downstream service.
    pub fn apply_signal(&self, signal: &BackpressureSignal) {
        match signal {
            BackpressureSignal::Throttle { max_rate } => {
                self.allowed_rate.store(*max_rate, Ordering::Relaxed);
                tracing::debug!(target = %self.target, rate = max_rate, "upstream throttled");
            }
            BackpressureSignal::Pause => {
                self.paused.store(true, Ordering::Relaxed);
                tracing::warn!(target = %self.target, "upstream paused by downstream");
            }
            BackpressureSignal::Resume => {
                self.paused.store(false, Ordering::Relaxed);
                self.allowed_rate.store(0, Ordering::Relaxed);
                tracing::info!(target = %self.target, "upstream resumed");
            }
            BackpressureSignal::Drain => {
                self.paused.store(true, Ordering::Relaxed);
                tracing::info!(target = %self.target, "upstream draining");
            }
            _ => {}
        }
    }

    /// Check if sending is currently allowed.
    pub fn can_send(&self) -> bool {
        !self.paused.load(Ordering::Relaxed)
    }

    /// Get the current allowed rate (0 = unlimited).
    pub fn allowed_rate(&self) -> u64 {
        self.allowed_rate.load(Ordering::Relaxed)
    }
}
