//! VIL LLM Proxy — high-performance streaming LLM proxy.
//!
//! Sits between clients and LLM providers, handling:
//! - **Routing** — intelligent model selection (cost, latency, health)
//! - **Caching** — semantic response cache with TTL
//! - **Rate Limiting** — per-key token bucket rate limiter
//! - **Metrics** — request, token, cost, and latency tracking
//!
//! # Example
//!
//! ```rust,no_run
//! use vil_llm_proxy::proxy::ProxyConfig;
//! use vil_llm_proxy::router::RoutingStrategy;
//! use std::time::Duration;
//!
//! let proxy = ProxyConfig::new()
//!     .cache_ttl(Duration::from_secs(300))
//!     .rate_limit(100.0, 6000.0)
//!     .routing_strategy(RoutingStrategy::LeastCost)
//!     .add_model("openai", "gpt-4", 3.0)
//!     .add_model("anthropic", "claude-3", 1.5)
//!     .build();
//! ```

pub mod cache;
pub mod handlers;
pub mod metrics;
pub mod pipeline_sse;
pub mod plugin;
pub mod proxy;
pub mod rate_limiter;
pub mod router;
pub mod semantic;

pub use cache::ResponseCache;
pub use metrics::{MetricsSnapshot, ProxyMetrics};
pub use plugin::LlmProxyPlugin;
pub use proxy::{LlmProxy, ProxyConfig, ProxyError};
pub use rate_limiter::{RateLimitExceeded, RateLimiter, RateLimiterConfig};
pub use router::{ModelEndpoint, ModelRouter, RoutingStrategy};
pub use semantic::{ProxyFault, ProxyRequestEvent, ProxyState};
