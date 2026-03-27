//! Core LLM proxy — composes cache, rate limiter, router, and metrics.

use std::sync::Arc;
use std::time::{Duration, Instant};

use vil_llm::{ChatMessage, ChatResponse, LlmProvider};
use vil_llm::message::LlmError;

use crate::cache::ResponseCache;
use crate::metrics::ProxyMetrics;
use crate::rate_limiter::{RateLimiter, RateLimiterConfig, RateLimitExceeded};
use crate::router::{ModelEndpoint, ModelRouter, RoutingStrategy};

/// Proxy error types.
#[derive(Debug)]
pub enum ProxyError {
    /// Rate limit exceeded.
    RateLimited(RateLimitExceeded),
    /// No healthy model endpoints.
    NoHealthyEndpoints,
    /// Underlying LLM error.
    LlmError(LlmError),
    /// No provider set for the selected model.
    ProviderNotConfigured(String),
}

impl std::fmt::Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RateLimited(e) => write!(f, "proxy rate limited: {}", e),
            Self::NoHealthyEndpoints => write!(f, "no healthy model endpoints available"),
            Self::LlmError(e) => write!(f, "LLM error: {}", e),
            Self::ProviderNotConfigured(m) => write!(f, "no provider configured for model '{}'", m),
        }
    }
}

impl std::error::Error for ProxyError {}

/// Builder for configuring the LLM proxy.
pub struct ProxyConfig {
    cache_ttl: Duration,
    cache_max_entries: usize,
    rate_limit_config: RateLimiterConfig,
    routing_strategy: RoutingStrategy,
    endpoints: Vec<ModelEndpoint>,
}

impl ProxyConfig {
    /// Start building a proxy config.
    pub fn new() -> Self {
        Self {
            cache_ttl: Duration::from_secs(300),
            cache_max_entries: 1000,
            rate_limit_config: RateLimiterConfig::default(),
            routing_strategy: RoutingStrategy::LeastCost,
            endpoints: Vec::new(),
        }
    }

    /// Set cache TTL.
    pub fn cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Set maximum cache entries.
    pub fn cache_max_entries(mut self, max: usize) -> Self {
        self.cache_max_entries = max;
        self
    }

    /// Set rate limit (max burst tokens, tokens per minute).
    pub fn rate_limit(mut self, max_tokens: f64, tokens_per_minute: f64) -> Self {
        self.rate_limit_config = RateLimiterConfig {
            max_tokens,
            tokens_per_minute,
        };
        self
    }

    /// Set routing strategy.
    pub fn routing_strategy(mut self, strategy: RoutingStrategy) -> Self {
        self.routing_strategy = strategy;
        self
    }

    /// Add a model endpoint.
    pub fn add_model(mut self, provider: &str, model: &str, cost_per_1k_tokens: f64) -> Self {
        self.endpoints.push(ModelEndpoint::new(provider, model, cost_per_1k_tokens));
        self
    }

    /// Build the LlmProxy.
    pub fn build(self) -> LlmProxy {
        let mut router = ModelRouter::new(self.routing_strategy);
        for ep in self.endpoints {
            router.add_endpoint(ep);
        }

        LlmProxy {
            cache: ResponseCache::with_config(self.cache_ttl, self.cache_max_entries),
            rate_limiter: RateLimiter::with_config(self.rate_limit_config),
            router,
            metrics: Arc::new(ProxyMetrics::new()),
            provider: None,
        }
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Core LLM proxy — composes cache + rate limiter + router + metrics.
pub struct LlmProxy {
    cache: ResponseCache,
    rate_limiter: RateLimiter,
    router: ModelRouter,
    metrics: Arc<ProxyMetrics>,
    provider: Option<Arc<dyn LlmProvider>>,
}

impl LlmProxy {
    /// Create with default config.
    pub fn new() -> Self {
        ProxyConfig::new().build()
    }

    /// Create from a config builder.
    pub fn from_config(config: ProxyConfig) -> Self {
        config.build()
    }

    /// Set the LLM provider for forwarding requests.
    pub fn set_provider(&mut self, provider: Arc<dyn LlmProvider>) {
        self.provider = Some(provider);
    }

    /// Get a reference to the metrics.
    pub fn metrics(&self) -> &Arc<ProxyMetrics> {
        &self.metrics
    }

    /// Get a reference to the router.
    pub fn router(&self) -> &ModelRouter {
        &self.router
    }

    /// Process a chat request through the proxy pipeline.
    ///
    /// Pipeline: rate limit -> cache check -> route -> forward -> cache store -> metrics.
    pub async fn chat(
        &self,
        api_key: &str,
        messages: &[ChatMessage],
    ) -> Result<ChatResponse, ProxyError> {
        self.metrics.record_request();

        // 1. Rate limit check
        if let Err(e) = self.rate_limiter.check(api_key, 1.0) {
            self.metrics.record_rate_limited();
            return Err(ProxyError::RateLimited(e));
        }

        // 2. Cache check
        if let Some(cached) = self.cache.get(messages) {
            self.metrics.record_cache_hit();
            return Ok(cached.to_chat_response());
        }
        self.metrics.record_cache_miss();

        // 3. Route to best model
        let endpoint = self.router.select()
            .ok_or(ProxyError::NoHealthyEndpoints)?;
        let model_name = endpoint.model.clone();

        // 4. Forward to LLM provider
        let provider = self.provider.as_ref()
            .ok_or_else(|| ProxyError::ProviderNotConfigured(model_name.clone()))?;

        let start = Instant::now();
        let result = provider.chat(messages).await;
        let latency_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(response) => {
                // Update endpoint health
                endpoint.mark_success(latency_ms);

                // 5. Cache response
                self.cache.put(messages, &response);

                // 6. Update metrics
                if let Some(ref usage) = response.usage {
                    let cost_cents = ((usage.total_tokens as f64 / 1000.0)
                        * endpoint.cost_per_1k_tokens
                        * 100.0) as u64;
                    self.metrics.record_usage(usage.total_tokens as u64, cost_cents);
                }
                self.metrics.record_model_request(&model_name);

                Ok(response)
            }
            Err(e) => {
                endpoint.mark_failure();
                self.metrics.record_error();
                Err(ProxyError::LlmError(e))
            }
        }
    }
}

impl Default for LlmProxy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use vil_llm::Usage;

    /// Mock LLM provider for testing.
    struct MockProvider;

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn chat(&self, _messages: &[ChatMessage]) -> Result<ChatResponse, LlmError> {
            Ok(ChatResponse {
                content: "Mock response".to_string(),
                model: "mock-model".to_string(),
                tool_calls: None,
                usage: Some(Usage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                }),
                finish_reason: Some("stop".to_string()),
            })
        }

        fn model(&self) -> &str { "mock-model" }
        fn provider_name(&self) -> &str { "mock" }
    }

    /// Mock provider that always fails.
    struct FailingProvider;

    #[async_trait]
    impl LlmProvider for FailingProvider {
        async fn chat(&self, _messages: &[ChatMessage]) -> Result<ChatResponse, LlmError> {
            Err(LlmError::RequestFailed("mock failure".into()))
        }

        fn model(&self) -> &str { "fail-model" }
        fn provider_name(&self) -> &str { "mock" }
    }

    fn build_test_proxy(provider: Arc<dyn LlmProvider>) -> LlmProxy {
        let mut proxy = ProxyConfig::new()
            .cache_ttl(Duration::from_secs(60))
            .rate_limit(100.0, 6000.0)
            .routing_strategy(RoutingStrategy::LeastCost)
            .add_model("mock", "mock-model", 1.0)
            .build();
        proxy.set_provider(provider);
        proxy
    }

    #[tokio::test]
    async fn test_full_flow() {
        let proxy = build_test_proxy(Arc::new(MockProvider));
        let msgs = vec![ChatMessage::user("Hello!")];

        let resp = proxy.chat("key-1", &msgs).await.unwrap();
        assert_eq!(resp.content, "Mock response");

        let snap = proxy.metrics().snapshot();
        assert_eq!(snap.total_requests, 1);
        assert_eq!(snap.cache_misses, 1);
    }

    #[tokio::test]
    async fn test_cache_hit_on_second_call() {
        let proxy = build_test_proxy(Arc::new(MockProvider));
        let msgs = vec![ChatMessage::user("Hello!")];

        // First call — cache miss
        proxy.chat("key-1", &msgs).await.unwrap();
        // Second call — cache hit
        let resp = proxy.chat("key-1", &msgs).await.unwrap();
        assert_eq!(resp.finish_reason, Some("cache_hit".to_string()));

        let snap = proxy.metrics().snapshot();
        assert_eq!(snap.total_requests, 2);
        assert_eq!(snap.cache_hits, 1);
        assert_eq!(snap.cache_misses, 1);
    }

    #[tokio::test]
    async fn test_rate_limit_rejection() {
        let mut proxy = ProxyConfig::new()
            .rate_limit(1.0, 60.0) // burst of 1
            .add_model("mock", "mock-model", 1.0)
            .build();
        proxy.set_provider(Arc::new(MockProvider));

        let msgs = vec![ChatMessage::user("Hello!")];

        // First should succeed
        proxy.chat("key-1", &msgs).await.unwrap();
        // Second should be rate limited
        let result = proxy.chat("key-1", &msgs).await;
        assert!(matches!(result, Err(ProxyError::RateLimited(_))));

        let snap = proxy.metrics().snapshot();
        assert_eq!(snap.rate_limited, 1);
    }

    #[tokio::test]
    async fn test_no_healthy_endpoints() {
        let mut proxy = ProxyConfig::new().build();
        proxy.set_provider(Arc::new(MockProvider));

        let msgs = vec![ChatMessage::user("Hello!")];
        let result = proxy.chat("key-1", &msgs).await;
        assert!(matches!(result, Err(ProxyError::NoHealthyEndpoints)));
    }

    #[tokio::test]
    async fn test_provider_error_updates_health() {
        let proxy = build_test_proxy(Arc::new(FailingProvider));
        let msgs = vec![ChatMessage::user("Hello!")];

        let result = proxy.chat("key-1", &msgs).await;
        assert!(matches!(result, Err(ProxyError::LlmError(_))));

        let snap = proxy.metrics().snapshot();
        assert_eq!(snap.errors, 1);
    }

    #[tokio::test]
    async fn test_no_provider_configured() {
        let proxy = ProxyConfig::new()
            .add_model("mock", "mock-model", 1.0)
            .build();

        let msgs = vec![ChatMessage::user("Hello!")];
        let result = proxy.chat("key-1", &msgs).await;
        assert!(matches!(result, Err(ProxyError::ProviderNotConfigured(_))));
    }
}
