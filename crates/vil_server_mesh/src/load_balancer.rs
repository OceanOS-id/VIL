// =============================================================================
// VIL Server Mesh — Load Balancer & Canary Routing
// =============================================================================
//
// Client-side load balancing for upstream services.
// Strategies: round-robin, least-connections, weighted, canary.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use serde::Serialize;

/// Load balancing strategy.
#[derive(Debug, Clone)]
pub enum LbStrategy {
    /// Cycle through endpoints in order
    RoundRobin,
    /// Pick endpoint with fewest active connections
    LeastConnections,
    /// Weighted distribution (endpoint weights)
    Weighted(Vec<u32>),
    /// Canary: send X% traffic to canary endpoint
    Canary { canary_weight: u8 },
}

/// A backend endpoint for load balancing.
#[derive(Debug)]
pub struct LbEndpoint {
    pub address: String,
    pub weight: u32,
    pub active_connections: AtomicU64,
    pub total_requests: AtomicU64,
    pub is_canary: bool,
}

impl LbEndpoint {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            weight: 1,
            active_connections: AtomicU64::new(0),
            total_requests: AtomicU64::new(0),
            is_canary: false,
        }
    }

    pub fn with_weight(mut self, weight: u32) -> Self {
        self.weight = weight;
        self
    }

    pub fn canary(mut self) -> Self {
        self.is_canary = true;
        self
    }
}

/// Load balancer.
pub struct LoadBalancer {
    endpoints: Vec<LbEndpoint>,
    strategy: LbStrategy,
    counter: AtomicUsize,
}

impl LoadBalancer {
    pub fn new(endpoints: Vec<LbEndpoint>, strategy: LbStrategy) -> Self {
        Self {
            endpoints,
            strategy,
            counter: AtomicUsize::new(0),
        }
    }

    /// Select the next endpoint based on strategy.
    pub fn next(&self) -> Option<&LbEndpoint> {
        if self.endpoints.is_empty() {
            return None;
        }

        match &self.strategy {
            LbStrategy::RoundRobin => {
                let idx = self.counter.fetch_add(1, Ordering::Relaxed) % self.endpoints.len();
                Some(&self.endpoints[idx])
            }
            LbStrategy::LeastConnections => {
                self.endpoints.iter()
                    .min_by_key(|e| e.active_connections.load(Ordering::Relaxed))
            }
            LbStrategy::Weighted(weights) => {
                let total: u32 = weights.iter().sum();
                if total == 0 { return self.endpoints.first(); }
                let target = (self.counter.fetch_add(1, Ordering::Relaxed) as u32) % total;
                let mut acc = 0;
                for (i, w) in weights.iter().enumerate() {
                    acc += w;
                    if target < acc {
                        return self.endpoints.get(i);
                    }
                }
                self.endpoints.last()
            }
            LbStrategy::Canary { canary_weight } => {
                let roll = (self.counter.fetch_add(1, Ordering::Relaxed) % 100) as u8;
                if roll < *canary_weight {
                    // Route to canary
                    self.endpoints.iter().find(|e| e.is_canary).or(self.endpoints.first())
                } else {
                    // Route to stable
                    self.endpoints.iter().find(|e| !e.is_canary).or(self.endpoints.first())
                }
            }
        }
    }

    /// Get endpoint count.
    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }

    /// Get load balancer status.
    pub fn status(&self) -> LbStatus {
        LbStatus {
            strategy: format!("{:?}", self.strategy),
            endpoints: self.endpoints.iter().map(|e| EndpointStatus {
                address: e.address.clone(),
                active_connections: e.active_connections.load(Ordering::Relaxed),
                total_requests: e.total_requests.load(Ordering::Relaxed),
                weight: e.weight,
                is_canary: e.is_canary,
            }).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LbStatus {
    pub strategy: String,
    pub endpoints: Vec<EndpointStatus>,
}

#[derive(Debug, Serialize)]
pub struct EndpointStatus {
    pub address: String,
    pub active_connections: u64,
    pub total_requests: u64,
    pub weight: u32,
    pub is_canary: bool,
}
