// =============================================================================
// VIL Server — HTTP Client Pool
// =============================================================================
//
// Connection-pooled HTTP client for upstream service calls.
// Integrates with:
//   - CircuitBreaker (auto-open on failures)
//   - RetryPolicy (auto-retry on transient errors)
//   - Tri-Lane mesh (SHM for co-located, HTTP for remote)
//   - VilMetrics (auto-track upstream latency)

use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// HTTP client configuration.
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Request timeout
    pub request_timeout: Duration,
    /// Maximum connections per host
    pub max_connections_per_host: usize,
    /// Maximum idle connections
    pub max_idle_connections: usize,
    /// Idle timeout
    pub idle_timeout: Duration,
    /// Default headers
    pub default_headers: HashMap<String, String>,
    /// User-Agent header
    pub user_agent: String,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
            max_connections_per_host: 100,
            max_idle_connections: 20,
            idle_timeout: Duration::from_secs(90),
            default_headers: HashMap::new(),
            user_agent: "vil-server/0.1.0".to_string(),
        }
    }
}

/// HTTP client pool — manages connections to upstream services.
pub struct HttpClientPool {
    config: HttpClientConfig,
    /// Per-host request counts
    request_counts: dashmap::DashMap<String, AtomicU64>,
    /// Per-host latency tracking
    latencies: dashmap::DashMap<String, Vec<u64>>,
}

impl HttpClientPool {
    pub fn new(config: HttpClientConfig) -> Self {
        Self {
            config,
            request_counts: dashmap::DashMap::new(),
            latencies: dashmap::DashMap::new(),
        }
    }

    /// Record a request to a host.
    pub fn record_request(&self, host: &str, latency_us: u64) {
        self.request_counts
            .entry(host.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);

        self.latencies
            .entry(host.to_string())
            .or_insert_with(Vec::new)
            .push(latency_us);
    }

    /// Get statistics for a host.
    pub fn host_stats(&self, host: &str) -> Option<HostStats> {
        let count = self.request_counts.get(host)?
            .load(Ordering::Relaxed);

        let latencies = self.latencies.get(host)?;
        let avg_latency = if latencies.is_empty() {
            0
        } else {
            latencies.iter().sum::<u64>() / latencies.len() as u64
        };

        Some(HostStats {
            host: host.to_string(),
            total_requests: count,
            avg_latency_us: avg_latency,
        })
    }

    /// Get all tracked hosts.
    pub fn hosts(&self) -> Vec<String> {
        self.request_counts.iter().map(|e| e.key().clone()).collect()
    }

    pub fn config(&self) -> &HttpClientConfig {
        &self.config
    }
}

impl Default for HttpClientPool {
    fn default() -> Self {
        Self::new(HttpClientConfig::default())
    }
}

/// Per-host statistics.
#[derive(Debug, Clone, Serialize)]
pub struct HostStats {
    pub host: String,
    pub total_requests: u64,
    pub avg_latency_us: u64,
}
