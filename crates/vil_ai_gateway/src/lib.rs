//! VIL AI Gateway — intelligent LLM routing with health tracking,
//! automatic failover, cost-aware routing, and circuit breaker.
//!
//! # Overview
//!
//! `vil_ai_gateway` provides an `AiGateway` that sits in front of multiple
//! LLM providers (via `vil_llm::LlmProvider`) and adds:
//!
//! - **Per-model health tracking** — rolling-window error rate, latency percentiles
//! - **Circuit breaker** — per-model, trips after N consecutive failures, auto-recovers
//! - **Cost tracking** — per-model token cost, budget enforcement
//! - **Routing policies** — primary+failover, cost-optimized, latency-optimized, round-robin
//!
//! # Quick start
//!
//! ```rust,ignore
//! use vil_ai_gateway::{AiGateway, RoutingPolicy};
//!
//! let gw = AiGateway::builder()
//!     .provider("gpt-4", gpt4_provider)
//!     .provider("claude", claude_provider)
//!     .pricing("gpt-4", 0.03, 0.06)
//!     .pricing("claude", 0.015, 0.075)
//!     .routing(RoutingPolicy::CostOptimized)
//!     .build();
//!
//! let resp = gw.chat(&messages).await?;
//! println!("model={} cost=${:.4}", resp.model_used, resp.cost_usd);
//! ```

pub mod circuit_breaker;
pub mod config;
pub mod cost;
pub mod gateway;
pub mod health;
pub mod metrics;
pub mod semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

// Re-exports
pub use config::{GatewayConfig, RoutingPolicy};
pub use cost::{BudgetExceeded, Budget, CostTracker, ModelCost};
pub use gateway::{AiGateway, AiGatewayBuilder, GatewayError, GatewayResponse};
pub use health::{HealthStatus, HealthTracker, ModelHealth};
pub use circuit_breaker::CircuitBreaker;
pub use metrics::{GatewayMetrics, MetricsSnapshot};
pub use plugin::AiGatewayPlugin;
pub use semantic::{GatewayEvent, GatewayFault, GatewayState};
