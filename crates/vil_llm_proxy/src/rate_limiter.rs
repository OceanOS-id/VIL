//! Per-key token bucket rate limiter.
//!
//! Each API key gets its own token bucket with configurable max tokens and refill rate.

use std::fmt;
use std::time::Instant;

use dashmap::DashMap;

/// Rate limit exceeded error.
#[derive(Debug, Clone)]
pub struct RateLimitExceeded {
    pub key: String,
    pub requested: f64,
    pub available: f64,
    pub retry_after_ms: u64,
}

impl fmt::Display for RateLimitExceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rate limit exceeded for key '{}': requested {}, available {:.1}, retry after {}ms",
            self.key, self.requested, self.available, self.retry_after_ms
        )
    }
}

impl std::error::Error for RateLimitExceeded {}

/// Token bucket for a single key.
struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Refill tokens based on elapsed time.
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    /// Try to consume tokens. Returns Ok(()) if allowed, Err with retry time otherwise.
    fn try_consume(&mut self, requested: f64) -> Result<(), (f64, u64)> {
        self.refill();
        if self.tokens >= requested {
            self.tokens -= requested;
            Ok(())
        } else {
            let deficit = requested - self.tokens;
            let retry_after_ms = ((deficit / self.refill_rate) * 1000.0).ceil() as u64;
            Err((self.tokens, retry_after_ms))
        }
    }
}

/// Rate limiter configuration.
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// Maximum tokens per bucket (burst size).
    pub max_tokens: f64,
    /// Tokens refilled per minute.
    pub tokens_per_minute: f64,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            max_tokens: 100.0,
            tokens_per_minute: 60.0,
        }
    }
}

/// Per-key token bucket rate limiter.
pub struct RateLimiter {
    buckets: DashMap<String, TokenBucket>,
    config: RateLimiterConfig,
}

impl RateLimiter {
    /// Create a new rate limiter with default config.
    pub fn new() -> Self {
        Self {
            buckets: DashMap::new(),
            config: RateLimiterConfig::default(),
        }
    }

    /// Create a rate limiter with custom config.
    pub fn with_config(config: RateLimiterConfig) -> Self {
        Self {
            buckets: DashMap::new(),
            config,
        }
    }

    /// Check if a request is allowed for the given key.
    pub fn check(&self, key: &str, tokens_requested: f64) -> Result<(), RateLimitExceeded> {
        let refill_rate = self.config.tokens_per_minute / 60.0; // per second

        let mut bucket = self
            .buckets
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(self.config.max_tokens, refill_rate));

        bucket
            .try_consume(tokens_requested)
            .map_err(|(available, retry_after_ms)| RateLimitExceeded {
                key: key.to_string(),
                requested: tokens_requested,
                available,
                retry_after_ms,
            })
    }

    /// Number of tracked keys.
    pub fn tracked_keys(&self) -> usize {
        self.buckets.len()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_within_limit() {
        let limiter = RateLimiter::with_config(RateLimiterConfig {
            max_tokens: 10.0,
            tokens_per_minute: 60.0,
        });

        // Should allow requests within the burst
        assert!(limiter.check("key-1", 5.0).is_ok());
        assert!(limiter.check("key-1", 5.0).is_ok());
    }

    #[test]
    fn test_reject_over_limit() {
        let limiter = RateLimiter::with_config(RateLimiterConfig {
            max_tokens: 10.0,
            tokens_per_minute: 60.0,
        });

        assert!(limiter.check("key-1", 10.0).is_ok());
        let err = limiter.check("key-1", 1.0).unwrap_err();
        assert_eq!(err.key, "key-1");
        assert!(err.retry_after_ms > 0);
    }

    #[test]
    fn test_refill_over_time() {
        let limiter = RateLimiter::with_config(RateLimiterConfig {
            max_tokens: 10.0,
            tokens_per_minute: 6000.0, // 100/sec — fast refill for test
        });

        // Drain the bucket
        assert!(limiter.check("key-1", 10.0).is_ok());
        assert!(limiter.check("key-1", 1.0).is_err());

        // Wait for refill
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Should have refilled ~5 tokens (100/sec * 0.05s)
        assert!(limiter.check("key-1", 3.0).is_ok());
    }

    #[test]
    fn test_separate_keys() {
        let limiter = RateLimiter::with_config(RateLimiterConfig {
            max_tokens: 5.0,
            tokens_per_minute: 60.0,
        });

        assert!(limiter.check("key-a", 5.0).is_ok());
        assert!(limiter.check("key-a", 1.0).is_err());
        // Different key should still have tokens
        assert!(limiter.check("key-b", 5.0).is_ok());
        assert_eq!(limiter.tracked_keys(), 2);
    }

    #[test]
    fn test_rate_limit_error_display() {
        let err = RateLimitExceeded {
            key: "test".to_string(),
            requested: 10.0,
            available: 2.5,
            retry_after_ms: 500,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("test"));
        assert!(msg.contains("500ms"));
    }
}
