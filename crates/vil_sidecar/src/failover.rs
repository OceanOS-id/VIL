// =============================================================================
// Failover Dispatcher — Sidecar → backup sidecar → WASM fallback
// =============================================================================
//
// Attempts invocation in order:
//   1. Primary sidecar
//   2. Backup sidecar (if configured)
//   3. WASM fallback module (if configured)
//
// Circuit breaker prevents repeated calls to a failing sidecar.

use crate::dispatcher::{self, InvokeResponse};
use crate::registry::SidecarRegistry;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::Instant;

/// Circuit breaker state for a sidecar.
pub struct CircuitBreaker {
    failures: AtomicU64,
    is_open: AtomicBool,
    last_failure: std::sync::Mutex<Option<Instant>>,
    threshold: u64,
    cooldown_secs: u64,
}

impl CircuitBreaker {
    pub fn new(threshold: u64, cooldown_secs: u64) -> Self {
        Self {
            failures: AtomicU64::new(0),
            is_open: AtomicBool::new(false),
            last_failure: std::sync::Mutex::new(None),
            threshold,
            cooldown_secs,
        }
    }

    /// Check if the circuit is closed (requests allowed).
    pub fn is_closed(&self) -> bool {
        if !self.is_open.load(Ordering::Relaxed) {
            return true;
        }

        // Check if cooldown has elapsed → transition to half-open
        if let Ok(guard) = self.last_failure.lock() {
            if let Some(last) = *guard {
                if last.elapsed().as_secs() >= self.cooldown_secs {
                    // Cooldown elapsed, allow one attempt (half-open)
                    return true;
                }
            }
        }
        false
    }

    /// Record a successful call → reset circuit.
    pub fn record_success(&self) {
        self.failures.store(0, Ordering::Relaxed);
        self.is_open.store(false, Ordering::Relaxed);
    }

    /// Record a failed call → may trip the circuit.
    pub fn record_failure(&self) {
        let count = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        if count >= self.threshold {
            self.is_open.store(true, Ordering::Relaxed);
            if let Ok(mut guard) = self.last_failure.lock() {
                *guard = Some(Instant::now());
            }
            tracing::warn!(
                failures = count,
                threshold = self.threshold,
                "circuit breaker opened"
            );
        }
    }

    /// Current failure count.
    pub fn failure_count(&self) -> u64 {
        self.failures.load(Ordering::Relaxed)
    }

    /// Whether the circuit is currently open.
    pub fn is_open(&self) -> bool {
        self.is_open.load(Ordering::Relaxed)
    }
}

/// Invoke with failover: primary → backup sidecar → WASM fallback.
///
/// Uses circuit breaker to skip failing sidecars quickly.
pub async fn invoke_with_failover(
    registry: &SidecarRegistry,
    target: &str,
    method: &str,
    request_data: &[u8],
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<InvokeResponse, FailoverError> {
    // Step 1: Try primary sidecar (if circuit is closed)
    let skip_primary = circuit_breaker
        .map(|cb| !cb.is_closed())
        .unwrap_or(false);

    if !skip_primary {
        match dispatcher::invoke(registry, target, method, request_data).await {
            Ok(resp) => {
                if let Some(cb) = circuit_breaker {
                    cb.record_success();
                }
                return Ok(resp);
            }
            Err(e) => {
                if let Some(cb) = circuit_breaker {
                    cb.record_failure();
                }
                tracing::warn!(sidecar = %target, error = %e, "primary sidecar failed");
            }
        }
    }

    // Step 2: Try backup sidecar (if configured)
    let failover_config = registry
        .get(target)
        .and_then(|entry| entry.config.failover.clone());

    if let Some(ref fo) = failover_config {
        if let Some(ref backup) = fo.backup {
            tracing::info!(primary = %target, backup = %backup, "failing over to backup sidecar");
            match dispatcher::invoke(registry, backup, method, request_data).await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    tracing::warn!(backup = %backup, error = %e, "backup sidecar also failed");
                }
            }
        }

        // Step 3: WASM fallback (return info for caller to handle)
        if let Some(ref wasm_module) = fo.fallback_wasm {
            return Err(FailoverError::WasmFallback {
                module: wasm_module.clone(),
                original_target: target.to_string(),
            });
        }
    }

    Err(FailoverError::AllFailed {
        target: target.to_string(),
        method: method.to_string(),
    })
}

/// Failover-specific errors.
#[derive(Debug)]
pub enum FailoverError {
    /// All sidecars failed, no WASM fallback configured.
    AllFailed { target: String, method: String },
    /// All sidecars failed, but a WASM fallback is available.
    /// The caller should dispatch to the WASM module.
    WasmFallback { module: String, original_target: String },
}

impl std::fmt::Display for FailoverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AllFailed { target, method } => {
                write!(f, "all sidecars failed for '{}.{}', no fallback", target, method)
            }
            Self::WasmFallback { module, original_target } => {
                write!(f, "sidecars for '{}' failed, WASM fallback: {}", original_target, module)
            }
        }
    }
}

impl std::error::Error for FailoverError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_closed_by_default() {
        let cb = CircuitBreaker::new(3, 30);
        assert!(cb.is_closed());
        assert!(!cb.is_open());
    }

    #[test]
    fn test_circuit_breaker_opens_after_threshold() {
        let cb = CircuitBreaker::new(3, 30);
        cb.record_failure();
        cb.record_failure();
        assert!(cb.is_closed()); // 2 < 3

        cb.record_failure(); // 3 >= 3 → opens
        assert!(cb.is_open());
        assert!(!cb.is_closed());
    }

    #[test]
    fn test_circuit_breaker_resets_on_success() {
        let cb = CircuitBreaker::new(2, 30);
        cb.record_failure();
        cb.record_failure(); // opens
        assert!(cb.is_open());

        cb.record_success(); // resets
        assert!(cb.is_closed());
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_failover_error_display() {
        let err = FailoverError::AllFailed {
            target: "fraud".into(),
            method: "check".into(),
        };
        assert!(err.to_string().contains("fraud"));

        let err = FailoverError::WasmFallback {
            module: "fraud_basic.wasm".into(),
            original_target: "fraud".into(),
        };
        assert!(err.to_string().contains("fraud_basic.wasm"));
    }
}
