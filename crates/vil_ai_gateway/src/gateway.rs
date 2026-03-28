use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use dashmap::DashMap;
use serde::Serialize;
use vil_llm::{LlmProvider, ChatMessage};
use vil_macros::{VilAiEvent, VilAiFault};

use crate::circuit_breaker::CircuitBreaker;
use crate::config::{GatewayConfig, RoutingPolicy};
use crate::cost::{CostTracker, ModelCost};
use crate::health::{HealthTracker, ModelHealth};
use crate::metrics::GatewayMetrics;
use vil_log::app_log;

/// Error type for gateway operations.
#[derive(Debug, Serialize, VilAiFault)]
pub enum GatewayError {
    /// All providers failed.
    AllProvidersFailed {
        attempts: Vec<(String, String)>,
    },
    /// No healthy providers available.
    NoHealthyProviders,
    /// Budget exceeded.
    BudgetExceeded(String),
    /// Timeout.
    Timeout,
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AllProvidersFailed { attempts } => {
                write!(f, "all providers failed: ")?;
                for (model, err) in attempts {
                    write!(f, "[{}:{}] ", model, err)?;
                }
                Ok(())
            }
            Self::NoHealthyProviders => write!(f, "no healthy providers available"),
            Self::BudgetExceeded(msg) => write!(f, "budget exceeded: {}", msg),
            Self::Timeout => write!(f, "gateway request timeout"),
        }
    }
}

impl std::error::Error for GatewayError {}

/// Response from the gateway including metadata.
#[derive(Debug, Serialize, VilAiEvent)]
pub struct GatewayResponse {
    pub content: String,
    pub model_used: String,
    pub latency_ms: u64,
    pub cost_usd: f64,
    pub attempts: u32,
}

/// The core AI Gateway with health tracking, circuit breakers, cost tracking, and failover.
pub struct AiGateway {
    providers: Vec<(String, Arc<dyn LlmProvider>)>,
    health: HealthTracker,
    breakers: DashMap<String, CircuitBreaker>,
    cost: CostTracker,
    config: GatewayConfig,
    metrics: GatewayMetrics,
    round_robin_idx: AtomicUsize,
}

/// Builder for constructing an AiGateway.
pub struct AiGatewayBuilder {
    providers: Vec<(String, Arc<dyn LlmProvider>)>,
    config: GatewayConfig,
    cost: CostTracker,
}

impl AiGatewayBuilder {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            config: GatewayConfig::default(),
            cost: CostTracker::new(),
        }
    }

    /// Add a provider with its model name.
    pub fn provider(mut self, name: impl Into<String>, provider: Arc<dyn LlmProvider>) -> Self {
        self.providers.push((name.into(), provider));
        self
    }

    /// Set gateway configuration.
    pub fn config(mut self, config: GatewayConfig) -> Self {
        self.config = config;
        self
    }

    /// Set routing policy.
    pub fn routing(mut self, policy: RoutingPolicy) -> Self {
        self.config.routing = policy;
        self
    }

    /// Set model pricing.
    pub fn pricing(self, model: &str, input_per_1k: f64, output_per_1k: f64) -> Self {
        self.cost.set_model_pricing(model, input_per_1k, output_per_1k);
        self
    }

    /// Build the gateway.
    pub fn build(self) -> AiGateway {
        let breakers = DashMap::new();
        for (name, _) in &self.providers {
            breakers.insert(
                name.clone(),
                CircuitBreaker::new(
                    self.config.circuit_breaker_threshold,
                    self.config.circuit_breaker_timeout_ms,
                ),
            );
        }
        AiGateway {
            providers: self.providers,
            health: HealthTracker::new(),
            breakers,
            cost: self.cost,
            config: self.config,
            metrics: GatewayMetrics::new(),
            round_robin_idx: AtomicUsize::new(0),
        }
    }
}

impl Default for AiGatewayBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AiGateway {
    /// Create a builder.
    pub fn builder() -> AiGatewayBuilder {
        AiGatewayBuilder::new()
    }

    /// Route and execute a chat request with health checks, circuit breakers,
    /// failover, and cost tracking.
    pub async fn chat(&self, messages: &[ChatMessage]) -> Result<GatewayResponse, GatewayError> {
        self.metrics.record_request();

        let ordered = self.route_order();
        if ordered.is_empty() {
            return Err(GatewayError::NoHealthyProviders);
        }

        let mut attempts: Vec<(String, String)> = Vec::new();
        let mut attempt_count = 0u32;

        for (name, provider) in &ordered {
            // Check circuit breaker
            if let Some(cb) = self.breakers.get(name) {
                if !cb.can_proceed() {
                    self.metrics.record_circuit_rejection();
                    app_log!(Debug, "gateway_circuit_breaker", { model: name.clone(), event: "open_skip" });
                    attempts.push((name.clone(), "circuit_breaker_open".to_string()));
                    continue;
                }
            }

            attempt_count += 1;
            if attempt_count > 1 {
                self.metrics.record_failover();
            }

            let start = Instant::now();
            let result = tokio::time::timeout(
                std::time::Duration::from_millis(self.config.timeout_ms),
                provider.chat(messages),
            )
            .await;

            let latency_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok(Ok(response)) => {
                    // Success
                    self.health.record_success(name, latency_ms);
                    if let Some(cb) = self.breakers.get(name) {
                        cb.record_success();
                    }
                    self.metrics.record_success();

                    // Track cost
                    let cost_usd = if let Some(usage) = &response.usage {
                        self.cost.record_usage(
                            name,
                            usage.prompt_tokens,
                            usage.completion_tokens,
                        )
                    } else {
                        0.0
                    };

                    return Ok(GatewayResponse {
                        content: response.content,
                        model_used: name.clone(),
                        latency_ms,
                        cost_usd,
                        attempts: attempt_count,
                    });
                }
                Ok(Err(err)) => {
                    let err_str = err.to_string();
                    app_log!(Warn, "gateway_provider_failed", { model: name.clone(), error: err_str.clone() });
                    self.health.record_failure(name, &err_str);
                    if let Some(cb) = self.breakers.get(name) {
                        cb.record_failure();
                    }
                    attempts.push((name.clone(), err_str));
                }
                Err(_) => {
                    app_log!(Warn, "gateway_provider_timeout", { model: name.clone() });
                    self.health.record_failure(name, "timeout");
                    if let Some(cb) = self.breakers.get(name) {
                        cb.record_failure();
                    }
                    attempts.push((name.clone(), "timeout".to_string()));
                }
            }
        }

        self.metrics.record_failure();
        Err(GatewayError::AllProvidersFailed { attempts })
    }

    /// Get health snapshots for all models.
    pub fn health(&self) -> Vec<ModelHealth> {
        self.health.get_all()
    }

    /// Get cost summary for all models.
    pub fn cost_summary(&self) -> Vec<ModelCost> {
        self.providers
            .iter()
            .filter_map(|(name, _)| self.cost.get_cost(name))
            .collect()
    }

    /// Get gateway metrics snapshot.
    pub fn metrics(&self) -> crate::metrics::MetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Get the cost tracker reference.
    pub fn cost_tracker(&self) -> &CostTracker {
        &self.cost
    }

    /// Determine provider ordering based on routing policy.
    fn route_order(&self) -> Vec<(String, Arc<dyn LlmProvider>)> {
        match &self.config.routing {
            RoutingPolicy::PrimaryWithFailover => {
                // Use provider list order; skip unhealthy at the call site via circuit breaker
                self.providers.clone()
            }
            RoutingPolicy::CostOptimized => {
                let mut ordered = self.providers.clone();
                ordered.sort_by(|a, b| {
                    let cost_a = self.cost.get_cost(&a.0).map(|c| c.cost_per_1k_input + c.cost_per_1k_output).unwrap_or(f64::MAX);
                    let cost_b = self.cost.get_cost(&b.0).map(|c| c.cost_per_1k_input + c.cost_per_1k_output).unwrap_or(f64::MAX);
                    cost_a.partial_cmp(&cost_b).unwrap_or(std::cmp::Ordering::Equal)
                });
                ordered
            }
            RoutingPolicy::LatencyOptimized => {
                let mut ordered = self.providers.clone();
                ordered.sort_by(|a, b| {
                    let lat_a = self.health.get_health(&a.0).avg_latency_ms;
                    let lat_b = self.health.get_health(&b.0).avg_latency_ms;
                    lat_a.partial_cmp(&lat_b).unwrap_or(std::cmp::Ordering::Equal)
                });
                ordered
            }
            RoutingPolicy::RoundRobin => {
                let len = self.providers.len();
                if len == 0 {
                    return vec![];
                }
                let start = self.round_robin_idx.fetch_add(1, Ordering::Relaxed) % len;
                let mut ordered = Vec::with_capacity(len);
                for i in 0..len {
                    let idx = (start + i) % len;
                    ordered.push(self.providers[idx].clone());
                }
                ordered
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use vil_llm::{ChatResponse, Usage};
    use vil_llm::message::LlmError;

    /// Mock provider that returns fixed content.
    struct MockProvider {
        name: String,
        response: Result<String, String>,
        usage: Option<Usage>,
    }

    impl MockProvider {
        fn ok(name: &str, content: &str) -> Arc<dyn LlmProvider> {
            Arc::new(Self {
                name: name.to_string(),
                response: Ok(content.to_string()),
                usage: Some(Usage {
                    prompt_tokens: 100,
                    completion_tokens: 50,
                    total_tokens: 150,
                }),
            })
        }

        fn err(name: &str, error: &str) -> Arc<dyn LlmProvider> {
            Arc::new(Self {
                name: name.to_string(),
                response: Err(error.to_string()),
                usage: None,
            })
        }
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn chat(&self, _messages: &[ChatMessage]) -> Result<ChatResponse, LlmError> {
            match &self.response {
                Ok(content) => Ok(ChatResponse {
                    content: content.clone(),
                    model: self.name.clone(),
                    tool_calls: None,
                    usage: self.usage.clone(),
                    finish_reason: Some("stop".to_string()),
                }),
                Err(e) => Err(LlmError::RequestFailed(e.clone())),
            }
        }

        fn model(&self) -> &str {
            &self.name
        }

        fn provider_name(&self) -> &str {
            "noop"
        }
    }

    #[tokio::test]
    async fn test_primary_with_failover_success() {
        let gw = AiGateway::builder()
            .provider("primary", MockProvider::ok("primary", "hello from primary"))
            .provider("fallback", MockProvider::ok("fallback", "hello from fallback"))
            .build();

        let msgs = vec![ChatMessage::user("hi")];
        let resp = gw.chat(&msgs).await.unwrap();
        assert_eq!(resp.model_used, "primary");
        assert_eq!(resp.content, "hello from primary");
        assert_eq!(resp.attempts, 1);
    }

    #[tokio::test]
    async fn test_failover_on_primary_failure() {
        let gw = AiGateway::builder()
            .provider("primary", MockProvider::err("primary", "down"))
            .provider("fallback", MockProvider::ok("fallback", "hello from fallback"))
            .build();

        let msgs = vec![ChatMessage::user("hi")];
        let resp = gw.chat(&msgs).await.unwrap();
        assert_eq!(resp.model_used, "fallback");
        assert_eq!(resp.content, "hello from fallback");
        assert_eq!(resp.attempts, 2);
    }

    #[tokio::test]
    async fn test_all_providers_fail() {
        let gw = AiGateway::builder()
            .provider("a", MockProvider::err("a", "err_a"))
            .provider("b", MockProvider::err("b", "err_b"))
            .build();

        let msgs = vec![ChatMessage::user("hi")];
        let result = gw.chat(&msgs).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            GatewayError::AllProvidersFailed { attempts } => {
                assert_eq!(attempts.len(), 2);
            }
            _ => panic!("expected AllProvidersFailed"),
        }
    }

    #[tokio::test]
    async fn test_cost_optimized_routing() {
        let gw = AiGateway::builder()
            .provider("expensive", MockProvider::ok("expensive", "exp"))
            .provider("cheap", MockProvider::ok("cheap", "chp"))
            .pricing("expensive", 0.10, 0.20)
            .pricing("cheap", 0.001, 0.002)
            .routing(RoutingPolicy::CostOptimized)
            .build();

        let msgs = vec![ChatMessage::user("hi")];
        let resp = gw.chat(&msgs).await.unwrap();
        assert_eq!(resp.model_used, "cheap");
    }

    #[tokio::test]
    async fn test_circuit_breaker_skips_open_model() {
        let config = GatewayConfig {
            circuit_breaker_threshold: 2,
            circuit_breaker_timeout_ms: 60_000, // long timeout so it stays open
            ..Default::default()
        };

        let gw = AiGateway::builder()
            .provider("flaky", MockProvider::err("flaky", "fail"))
            .provider("stable", MockProvider::ok("stable", "ok"))
            .config(config)
            .build();

        let msgs = vec![ChatMessage::user("hi")];

        // First two requests: flaky fails, then falls through to stable
        gw.chat(&msgs).await.unwrap(); // flaky fails → stable answers
        gw.chat(&msgs).await.unwrap(); // flaky fails again (2nd failure) → trips open → stable answers

        // Third request: circuit for flaky is open, goes straight to stable
        let resp = gw.chat(&msgs).await.unwrap();
        assert_eq!(resp.model_used, "stable");

        // Check metrics — failovers occurred
        let m = gw.metrics();
        assert!(m.total_failovers > 0);
    }

    #[tokio::test]
    async fn test_round_robin_routing() {
        let gw = AiGateway::builder()
            .provider("a", MockProvider::ok("a", "resp_a"))
            .provider("b", MockProvider::ok("b", "resp_b"))
            .routing(RoutingPolicy::RoundRobin)
            .build();

        let msgs = vec![ChatMessage::user("hi")];
        let r1 = gw.chat(&msgs).await.unwrap();
        let r2 = gw.chat(&msgs).await.unwrap();
        // Should alternate
        assert_ne!(r1.model_used, r2.model_used);
    }

    #[tokio::test]
    async fn test_cost_tracking() {
        let gw = AiGateway::builder()
            .provider("gpt-4", MockProvider::ok("gpt-4", "response"))
            .pricing("gpt-4", 0.03, 0.06)
            .build();

        let msgs = vec![ChatMessage::user("hi")];
        let resp = gw.chat(&msgs).await.unwrap();
        assert!(resp.cost_usd > 0.0);

        let summary = gw.cost_summary();
        assert_eq!(summary.len(), 1);
        assert!(summary[0].total_cost_usd > 0.0);
    }

    #[tokio::test]
    async fn test_no_providers_returns_error() {
        let gw = AiGateway::builder().build();
        let msgs = vec![ChatMessage::user("hi")];
        let result = gw.chat(&msgs).await;
        assert!(result.is_err());
    }
}
