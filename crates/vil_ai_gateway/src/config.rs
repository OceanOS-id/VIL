use serde::{Serialize, Deserialize};

/// Routing policy for the AI Gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingPolicy {
    /// Always use primary, failover to next on error.
    PrimaryWithFailover,
    /// Route to cheapest healthy model.
    CostOptimized,
    /// Route to lowest latency healthy model.
    LatencyOptimized,
    /// Round-robin across healthy models.
    RoundRobin,
}

impl Default for RoutingPolicy {
    fn default() -> Self {
        Self::PrimaryWithFailover
    }
}

/// Configuration for the AI Gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Routing policy.
    pub routing: RoutingPolicy,
    /// Number of consecutive failures before circuit breaker trips.
    pub circuit_breaker_threshold: u32,
    /// Milliseconds to wait before attempting half-open.
    pub circuit_breaker_timeout_ms: u64,
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            routing: RoutingPolicy::PrimaryWithFailover,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout_ms: 30_000,
            timeout_ms: 30_000,
        }
    }
}
