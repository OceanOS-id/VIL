//! Intelligent model routing — cost, latency, health-weighted.
//!
//! Routes requests to the best available model endpoint based on configurable strategy.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Routing strategy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RoutingStrategy {
    /// Route to the cheapest endpoint.
    LeastCost,
    /// Route to the endpoint with lowest average latency.
    LeastLatency,
    /// Round-robin across all healthy endpoints.
    RoundRobin,
    /// Weighted by health score (higher health = more traffic).
    HealthWeighted,
}

/// A model endpoint with health and performance tracking.
pub struct ModelEndpoint {
    pub provider: String,
    pub model: String,
    pub health: AtomicU32, // 0-100 health score
    pub total_requests: AtomicU64,
    pub avg_latency_ms: AtomicU64,
    pub cost_per_1k_tokens: f64,
}

impl ModelEndpoint {
    /// Create a new endpoint (starts fully healthy).
    pub fn new(
        provider: impl Into<String>,
        model: impl Into<String>,
        cost_per_1k_tokens: f64,
    ) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            health: AtomicU32::new(100),
            total_requests: AtomicU64::new(0),
            avg_latency_ms: AtomicU64::new(0),
            cost_per_1k_tokens,
        }
    }

    /// Mark a successful request, updating latency.
    pub fn mark_success(&self, latency_ms: u64) {
        let total = self.total_requests.fetch_add(1, Ordering::Relaxed) + 1;
        let old_avg = self.avg_latency_ms.load(Ordering::Relaxed);
        // Exponential moving average with more weight on recent
        let new_avg = if total <= 1 {
            latency_ms
        } else {
            (old_avg * 7 + latency_ms * 3) / 10
        };
        self.avg_latency_ms.store(new_avg, Ordering::Relaxed);

        // Improve health (cap at 100)
        let h = self.health.load(Ordering::Relaxed);
        if h < 100 {
            self.health.store((h + 5).min(100), Ordering::Relaxed);
        }
    }

    /// Mark a failed request.
    pub fn mark_failure(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        let h = self.health.load(Ordering::Relaxed);
        self.health.store(h.saturating_sub(20), Ordering::Relaxed);
    }

    /// Current health score (0-100).
    pub fn health_score(&self) -> u32 {
        self.health.load(Ordering::Relaxed)
    }

    /// Check if endpoint is considered healthy (health > 0).
    pub fn is_healthy(&self) -> bool {
        self.health_score() > 0
    }
}

/// Model router with multiple backends.
pub struct ModelRouter {
    endpoints: Vec<ModelEndpoint>,
    strategy: RoutingStrategy,
    round_robin_idx: AtomicU64,
}

impl ModelRouter {
    /// Create a new router with the given strategy.
    pub fn new(strategy: RoutingStrategy) -> Self {
        Self {
            endpoints: Vec::new(),
            strategy,
            round_robin_idx: AtomicU64::new(0),
        }
    }

    /// Add an endpoint.
    pub fn add_endpoint(&mut self, endpoint: ModelEndpoint) {
        self.endpoints.push(endpoint);
    }

    /// Number of registered endpoints.
    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }

    /// Select the best endpoint based on the current strategy.
    /// Returns None if no healthy endpoints are available.
    pub fn select(&self) -> Option<&ModelEndpoint> {
        let healthy: Vec<&ModelEndpoint> =
            self.endpoints.iter().filter(|e| e.is_healthy()).collect();

        if healthy.is_empty() {
            return None;
        }

        match self.strategy {
            RoutingStrategy::LeastCost => healthy.into_iter().min_by(|a, b| {
                a.cost_per_1k_tokens
                    .partial_cmp(&b.cost_per_1k_tokens)
                    .unwrap()
            }),
            RoutingStrategy::LeastLatency => healthy
                .into_iter()
                .min_by_key(|e| e.avg_latency_ms.load(Ordering::Relaxed)),
            RoutingStrategy::RoundRobin => {
                let idx = self.round_robin_idx.fetch_add(1, Ordering::Relaxed) as usize;
                Some(healthy[idx % healthy.len()])
            }
            RoutingStrategy::HealthWeighted => {
                // Pick the endpoint with the highest health score
                healthy.into_iter().max_by_key(|e| e.health_score())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_router(strategy: RoutingStrategy) -> ModelRouter {
        let mut router = ModelRouter::new(strategy);
        router.add_endpoint(ModelEndpoint::new("openai", "gpt-4", 3.0));
        router.add_endpoint(ModelEndpoint::new("anthropic", "claude-3", 1.5));
        router.add_endpoint(ModelEndpoint::new("ollama", "llama3", 0.0));
        router
    }

    #[test]
    fn test_least_cost_routing() {
        let router = setup_router(RoutingStrategy::LeastCost);
        let selected = router.select().unwrap();
        assert_eq!(selected.provider, "ollama"); // cheapest
    }

    #[test]
    fn test_least_latency_routing() {
        let router = setup_router(RoutingStrategy::LeastLatency);

        // Set different latencies
        router.endpoints[0]
            .avg_latency_ms
            .store(200, Ordering::Relaxed);
        router.endpoints[1]
            .avg_latency_ms
            .store(50, Ordering::Relaxed);
        router.endpoints[2]
            .avg_latency_ms
            .store(100, Ordering::Relaxed);

        let selected = router.select().unwrap();
        assert_eq!(selected.provider, "anthropic"); // lowest latency
    }

    #[test]
    fn test_round_robin_routing() {
        let router = setup_router(RoutingStrategy::RoundRobin);

        let first = router.select().unwrap().provider.clone();
        let second = router.select().unwrap().provider.clone();
        let _third = router.select().unwrap().provider.clone();

        // All three should be different (cycling through)
        assert_ne!(first, second);
        // After 3 selections, it wraps around
        let fourth = router.select().unwrap().provider.clone();
        assert_eq!(first, fourth);
    }

    #[test]
    fn test_health_weighted_routing() {
        let router = setup_router(RoutingStrategy::HealthWeighted);

        // Degrade health of first two
        router.endpoints[0].health.store(30, Ordering::Relaxed);
        router.endpoints[1].health.store(50, Ordering::Relaxed);
        // endpoints[2] stays at 100

        let selected = router.select().unwrap();
        assert_eq!(selected.provider, "ollama"); // healthiest
    }

    #[test]
    fn test_mark_success_updates_latency() {
        let ep = ModelEndpoint::new("openai", "gpt-4", 3.0);
        ep.mark_success(100);
        assert_eq!(ep.avg_latency_ms.load(Ordering::Relaxed), 100);
        assert_eq!(ep.total_requests.load(Ordering::Relaxed), 1);

        ep.mark_success(200);
        // EMA: (100*7 + 200*3)/10 = 130
        assert_eq!(ep.avg_latency_ms.load(Ordering::Relaxed), 130);
    }

    #[test]
    fn test_mark_failure_degrades_health() {
        let ep = ModelEndpoint::new("openai", "gpt-4", 3.0);
        assert_eq!(ep.health_score(), 100);

        ep.mark_failure();
        assert_eq!(ep.health_score(), 80);

        ep.mark_failure();
        assert_eq!(ep.health_score(), 60);
    }

    #[test]
    fn test_unhealthy_endpoints_excluded() {
        let mut router = ModelRouter::new(RoutingStrategy::LeastCost);
        router.add_endpoint(ModelEndpoint::new("openai", "gpt-4", 1.0));
        router.add_endpoint(ModelEndpoint::new("anthropic", "claude", 2.0));

        // Kill first endpoint
        router.endpoints[0].health.store(0, Ordering::Relaxed);

        let selected = router.select().unwrap();
        assert_eq!(selected.provider, "anthropic");
    }

    #[test]
    fn test_no_healthy_endpoints() {
        let mut router = ModelRouter::new(RoutingStrategy::LeastCost);
        router.add_endpoint(ModelEndpoint::new("openai", "gpt-4", 1.0));
        router.endpoints[0].health.store(0, Ordering::Relaxed);

        assert!(router.select().is_none());
    }
}
