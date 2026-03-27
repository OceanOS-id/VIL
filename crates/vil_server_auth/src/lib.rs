// =============================================================================
// VIL Server Auth — JWT validation, rate limiting, security middleware
// =============================================================================

pub mod jwt;
pub mod rate_limit;
pub mod circuit_breaker;
pub mod oauth2;
pub mod security;

// Sprint 7-9: Advanced Security
pub mod api_key;
pub mod ip_filter;
pub mod rbac;
pub mod csrf;
pub mod audit;
pub mod session;

pub use jwt::JwtAuth;
pub use rate_limit::RateLimit;
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
