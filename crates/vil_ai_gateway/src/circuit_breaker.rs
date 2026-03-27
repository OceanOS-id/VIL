use parking_lot::RwLock;
use std::time::Instant;

/// State of the circuit breaker.
#[derive(Debug, Clone)]
pub enum CircuitState {
    /// Normal operation — requests pass through.
    Closed { consecutive_failures: u32 },
    /// Rejecting all requests — waiting for recovery timeout.
    Open { since: Instant },
    /// Allowing one test request to check if the service recovered.
    HalfOpen { test_in_progress: bool },
}

/// Circuit breaker per model — trips after N consecutive failures,
/// recovers after a timeout, tests with a single request in half-open state.
pub struct CircuitBreaker {
    state: RwLock<CircuitState>,
    failure_threshold: u32,
    recovery_timeout_ms: u64,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, recovery_timeout_ms: u64) -> Self {
        Self {
            state: RwLock::new(CircuitState::Closed {
                consecutive_failures: 0,
            }),
            failure_threshold,
            recovery_timeout_ms,
        }
    }

    /// Check if a request can proceed through the circuit breaker.
    pub fn can_proceed(&self) -> bool {
        let now = Instant::now();
        // First check with read lock
        {
            let state = self.state.read();
            match &*state {
                CircuitState::Closed { .. } => return true,
                CircuitState::Open { since } => {
                    let elapsed = now.duration_since(*since).as_millis() as u64;
                    if elapsed < self.recovery_timeout_ms {
                        return false;
                    }
                    // Timeout elapsed — need to transition to HalfOpen (upgrade to write)
                }
                CircuitState::HalfOpen { test_in_progress } => {
                    return !*test_in_progress;
                }
            }
        }
        // Upgrade to write lock for Open → HalfOpen transition
        let mut state = self.state.write();
        if let CircuitState::Open { since } = &*state {
            let elapsed = now.duration_since(*since).as_millis() as u64;
            if elapsed >= self.recovery_timeout_ms {
                *state = CircuitState::HalfOpen {
                    test_in_progress: true,
                };
                return true;
            }
        }
        false
    }

    /// Record a successful request — reset to Closed.
    pub fn record_success(&self) {
        let mut state = self.state.write();
        *state = CircuitState::Closed {
            consecutive_failures: 0,
        };
    }

    /// Record a failed request — increment failures, potentially trip to Open.
    pub fn record_failure(&self) {
        let mut state = self.state.write();
        match &*state {
            CircuitState::Closed {
                consecutive_failures,
            } => {
                let new_count = consecutive_failures + 1;
                if new_count >= self.failure_threshold {
                    *state = CircuitState::Open {
                        since: Instant::now(),
                    };
                } else {
                    *state = CircuitState::Closed {
                        consecutive_failures: new_count,
                    };
                }
            }
            CircuitState::HalfOpen { .. } => {
                // Test request failed — back to Open
                *state = CircuitState::Open {
                    since: Instant::now(),
                };
            }
            CircuitState::Open { .. } => {
                // Already open, nothing to do
            }
        }
    }

    /// Return current state as a string.
    pub fn state(&self) -> String {
        let state = self.state.read();
        match &*state {
            CircuitState::Closed { .. } => "closed".to_string(),
            CircuitState::Open { .. } => "open".to_string(),
            CircuitState::HalfOpen { .. } => "half-open".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_closed_allows_requests() {
        let cb = CircuitBreaker::new(3, 1000);
        assert!(cb.can_proceed());
        assert_eq!(cb.state(), "closed");
    }

    #[test]
    fn test_trips_to_open_after_threshold() {
        let cb = CircuitBreaker::new(3, 1000);
        cb.record_failure();
        cb.record_failure();
        assert!(cb.can_proceed()); // still closed at 2 failures
        cb.record_failure(); // 3rd failure → open
        assert_eq!(cb.state(), "open");
        assert!(!cb.can_proceed());
    }

    #[test]
    fn test_success_resets_failures() {
        let cb = CircuitBreaker::new(3, 1000);
        cb.record_failure();
        cb.record_failure();
        cb.record_success(); // reset
        assert_eq!(cb.state(), "closed");
        cb.record_failure();
        assert!(cb.can_proceed()); // only 1 failure after reset
    }

    #[test]
    fn test_open_to_half_open_after_timeout() {
        let cb = CircuitBreaker::new(2, 50); // 50ms timeout
        cb.record_failure();
        cb.record_failure(); // trips to open
        assert_eq!(cb.state(), "open");
        assert!(!cb.can_proceed());

        std::thread::sleep(Duration::from_millis(60));
        assert!(cb.can_proceed()); // should transition to half-open
        assert_eq!(cb.state(), "half-open");
    }

    #[test]
    fn test_half_open_success_closes() {
        let cb = CircuitBreaker::new(2, 50);
        cb.record_failure();
        cb.record_failure();
        std::thread::sleep(Duration::from_millis(60));
        assert!(cb.can_proceed()); // half-open
        cb.record_success(); // test passed → closed
        assert_eq!(cb.state(), "closed");
    }

    #[test]
    fn test_half_open_failure_reopens() {
        let cb = CircuitBreaker::new(2, 50);
        cb.record_failure();
        cb.record_failure();
        std::thread::sleep(Duration::from_millis(60));
        assert!(cb.can_proceed()); // half-open
        cb.record_failure(); // test failed → open again
        assert_eq!(cb.state(), "open");
    }
}
