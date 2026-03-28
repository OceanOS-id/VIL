// =============================================================================
// VIL Server Auth — Circuit Breaker Middleware
// =============================================================================
//
// Implements the circuit breaker pattern for upstream service calls.
// Integrated with the Control Lane for automatic state propagation.
//
// States:
//   Closed  → normal operation, requests pass through
//   Open    → failures exceeded threshold, requests rejected immediately
//   HalfOpen → testing if upstream has recovered
//
// Transitions:
//   Closed  → Open:     failure_count >= threshold within window
//   Open    → HalfOpen: cooldown period elapsed
//   HalfOpen → Closed:  test request succeeded
//   HalfOpen → Open:    test request failed

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{Duration, Instant};
use vil_log::app_log;

/// Circuit breaker state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation — requests pass through
    Closed,
    /// Upstream is down — requests rejected immediately
    Open,
    /// Testing recovery — limited requests allowed
    HalfOpen,
}

/// Circuit breaker configuration.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: u64,
    /// Time window for counting failures
    pub failure_window: Duration,
    /// How long to wait before attempting recovery (HalfOpen)
    pub cooldown: Duration,
    /// Number of test requests allowed in HalfOpen state
    pub half_open_max_requests: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            failure_window: Duration::from_secs(60),
            cooldown: Duration::from_secs(30),
            half_open_max_requests: 3,
        }
    }
}

/// Circuit breaker for a specific upstream service.
///
/// Thread-safe — can be shared across handler tasks.
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: RwLock<CircuitState>,
    failure_count: AtomicU64,
    success_count: AtomicU64,
    half_open_attempts: AtomicU64,
    last_failure_time: RwLock<Option<Instant>>,
    opened_at: RwLock<Option<Instant>>,
    service_name: String,
}

impl CircuitBreaker {
    pub fn new(service_name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: RwLock::new(CircuitState::Closed),
            failure_count: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            half_open_attempts: AtomicU64::new(0),
            last_failure_time: RwLock::new(None),
            opened_at: RwLock::new(None),
            service_name: service_name.into(),
        }
    }

    /// Check if a request should be allowed through.
    ///
    /// Returns Ok(()) if allowed, Err(CircuitState) if rejected.
    pub fn check(&self) -> Result<(), CircuitState> {
        let state = *self.state.read().unwrap();

        match state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                // Check if cooldown has elapsed → transition to HalfOpen
                if let Some(opened_at) = *self.opened_at.read().unwrap() {
                    if opened_at.elapsed() >= self.config.cooldown {
                        *self.state.write().unwrap() = CircuitState::HalfOpen;
                        self.half_open_attempts.store(0, Ordering::Relaxed);
                        app_log!(Info, "circuit_breaker", { service: self.service_name.clone(), transition: "Open→HalfOpen" });
                        return Ok(());
                    }
                }
                Err(CircuitState::Open)
            }
            CircuitState::HalfOpen => {
                let attempts = self.half_open_attempts.fetch_add(1, Ordering::Relaxed);
                if attempts < self.config.half_open_max_requests {
                    Ok(())
                } else {
                    Err(CircuitState::HalfOpen)
                }
            }
        }
    }

    /// Record a successful request.
    pub fn record_success(&self) {
        let state = *self.state.read().unwrap();
        self.success_count.fetch_add(1, Ordering::Relaxed);

        if state == CircuitState::HalfOpen {
            // Recovery confirmed → close circuit
            *self.state.write().unwrap() = CircuitState::Closed;
            self.failure_count.store(0, Ordering::Relaxed);
            *self.opened_at.write().unwrap() = None;
            app_log!(Info, "circuit_breaker", { service: self.service_name.clone(), transition: "HalfOpen→Closed" });
        }
    }

    /// Record a failed request.
    pub fn record_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        *self.last_failure_time.write().unwrap() = Some(Instant::now());

        let state = *self.state.read().unwrap();

        match state {
            CircuitState::Closed => {
                if count >= self.config.failure_threshold {
                    *self.state.write().unwrap() = CircuitState::Open;
                    *self.opened_at.write().unwrap() = Some(Instant::now());
                    app_log!(Warn, "circuit_breaker", { service: self.service_name.clone(), transition: "Closed→Open", failures: count });
                }
            }
            CircuitState::HalfOpen => {
                // Recovery failed → back to Open
                *self.state.write().unwrap() = CircuitState::Open;
                *self.opened_at.write().unwrap() = Some(Instant::now());
                app_log!(Warn, "circuit_breaker", { service: self.service_name.clone(), transition: "HalfOpen→Open" });
            }
            CircuitState::Open => {}
        }
    }

    /// Get current circuit state.
    pub fn state(&self) -> CircuitState {
        *self.state.read().unwrap()
    }

    /// Get failure count.
    pub fn failure_count(&self) -> u64 {
        self.failure_count.load(Ordering::Relaxed)
    }

    /// Get success count.
    pub fn success_count(&self) -> u64 {
        self.success_count.load(Ordering::Relaxed)
    }

    /// Get the service name.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Reset the circuit breaker to Closed state.
    pub fn reset(&self) {
        *self.state.write().unwrap() = CircuitState::Closed;
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        *self.opened_at.write().unwrap() = None;
        app_log!(Info, "circuit_breaker", { service: self.service_name.clone(), event: "reset" });
    }

    /// Export status as JSON-compatible struct.
    pub fn status(&self) -> CircuitBreakerStatus {
        CircuitBreakerStatus {
            service: self.service_name.clone(),
            state: format!("{:?}", self.state()),
            failures: self.failure_count(),
            successes: self.success_count(),
            threshold: self.config.failure_threshold,
            cooldown_secs: self.config.cooldown.as_secs(),
        }
    }
}

/// Serializable circuit breaker status.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CircuitBreakerStatus {
    pub service: String,
    pub state: String,
    pub failures: u64,
    pub successes: u64,
    pub threshold: u64,
    pub cooldown_secs: u64,
}
