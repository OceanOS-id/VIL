// =============================================================================
// VIL Server — Retry Middleware (Outbound Requests)
// =============================================================================
//
// Provides retry logic for outbound HTTP calls to upstream services.
// Integrates with CircuitBreaker for coordinated failure handling.
//
// Strategies:
//   - Fixed delay (e.g., 100ms between retries)
//   - Exponential backoff (100ms → 200ms → 400ms → ...)
//   - Exponential backoff with jitter (recommended)

use std::time::Duration;

/// Retry strategy.
#[derive(Debug, Clone)]
pub enum RetryStrategy {
    /// Fixed delay between retries
    Fixed { delay: Duration },
    /// Exponential backoff: delay * 2^attempt
    ExponentialBackoff {
        initial_delay: Duration,
        max_delay: Duration,
    },
    /// Exponential backoff with random jitter (±25%)
    ExponentialBackoffJitter {
        initial_delay: Duration,
        max_delay: Duration,
    },
}

impl RetryStrategy {
    /// Calculate the delay for a given attempt (0-indexed).
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        match self {
            Self::Fixed { delay } => *delay,
            Self::ExponentialBackoff {
                initial_delay,
                max_delay,
            } => {
                let delay = initial_delay.as_millis() as u64 * 2u64.pow(attempt);
                Duration::from_millis(delay.min(max_delay.as_millis() as u64))
            }
            Self::ExponentialBackoffJitter {
                initial_delay,
                max_delay,
            } => {
                let base = initial_delay.as_millis() as u64 * 2u64.pow(attempt);
                let capped = base.min(max_delay.as_millis() as u64);
                // Add ±25% jitter
                let jitter = capped / 4;
                let nanos = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos() as u64;
                let jittered = capped.saturating_sub(jitter) + (nanos % (jitter * 2 + 1));
                Duration::from_millis(jittered.min(max_delay.as_millis() as u64))
            }
        }
    }
}

/// Retry policy configuration.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Retry strategy
    pub strategy: RetryStrategy,
    /// HTTP status codes that trigger a retry
    pub retryable_statuses: Vec<u16>,
    /// Whether to retry on connection errors
    pub retry_on_connection_error: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            strategy: RetryStrategy::ExponentialBackoffJitter {
                initial_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(5),
            },
            retryable_statuses: vec![502, 503, 504, 429],
            retry_on_connection_error: true,
        }
    }
}

impl RetryPolicy {
    pub fn new(max_retries: u32, strategy: RetryStrategy) -> Self {
        Self {
            max_retries,
            strategy,
            ..Default::default()
        }
    }

    /// Check if a status code is retryable.
    pub fn is_retryable_status(&self, status: u16) -> bool {
        self.retryable_statuses.contains(&status)
    }

    /// No retries.
    pub fn none() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }
}

/// Execute a function with retry logic.
///
/// # Example
/// ```ignore
/// use vil_server_core::retry::*;
///
/// async fn call_upstream() -> Result<String, String> {
///     Ok("response".to_string())
/// }
///
/// let policy = RetryPolicy::default();
/// let result = retry_async(&policy, || call_upstream()).await;
/// ```
pub async fn retry_async<F, Fut, T, E>(policy: &RetryPolicy, mut f: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_error = None;

    for attempt in 0..=policy.max_retries {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempt < policy.max_retries {
                    let delay = policy.strategy.delay_for_attempt(attempt);
                    {
                        use vil_log::app_log;
                        app_log!(Warn, "retry.attempt.failed", {
                            attempt: (attempt + 1) as u64,
                            max: policy.max_retries as u64,
                            delay_ms: delay.as_millis() as u64
                        });
                    }
                    tokio::time::sleep(delay).await;
                }
                last_error = Some(e);
            }
        }
    }

    Err(last_error.unwrap())
}
