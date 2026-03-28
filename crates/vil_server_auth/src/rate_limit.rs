// =============================================================================
// VIL Server Auth — Rate limiting middleware
// =============================================================================

use dashmap::DashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Token bucket rate limiter.
#[derive(Clone)]
pub struct RateLimit {
    /// Maximum requests per window
    max_requests: u64,
    /// Time window
    window: Duration,
    /// Per-IP buckets
    buckets: Arc<DashMap<IpAddr, Bucket>>,
}

struct Bucket {
    tokens: u64,
    last_refill: Instant,
}

impl RateLimit {
    /// Create a new rate limiter: max_requests per window duration.
    pub fn new(max_requests: u64, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            buckets: Arc::new(DashMap::new()),
        }
    }

    /// Create a rate limiter for per-IP limiting.
    /// Example: 100 requests per 60 seconds per IP.
    pub fn per_ip(max_requests: u64, window: Duration) -> Self {
        Self::new(max_requests, window)
    }

    /// Check if a request from the given IP is allowed.
    pub fn check(&self, ip: IpAddr) -> bool {
        let now = Instant::now();

        let mut entry = self.buckets.entry(ip).or_insert_with(|| Bucket {
            tokens: self.max_requests,
            last_refill: now,
        });

        let bucket = entry.value_mut();

        // Refill tokens if window has passed
        let elapsed = now.duration_since(bucket.last_refill);
        if elapsed >= self.window {
            bucket.tokens = self.max_requests;
            bucket.last_refill = now;
        }

        // Check and consume token
        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            true
        } else {
            false
        }
    }

    /// Get remaining tokens for an IP.
    pub fn remaining(&self, ip: IpAddr) -> u64 {
        self.buckets
            .get(&ip)
            .map(|b| b.tokens)
            .unwrap_or(self.max_requests)
    }

    /// Remove expired buckets (stale for > 2x window).
    /// Call periodically (e.g. every 60s via tokio::spawn) to prevent unbounded memory growth.
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        let max_age = self.window * 2;
        self.buckets.retain(|_, bucket| {
            now.duration_since(bucket.last_refill) < max_age
        });
    }

    /// Current number of tracked IPs.
    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }
}
