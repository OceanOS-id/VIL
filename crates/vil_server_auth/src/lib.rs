// =============================================================================
// VIL Server Auth — JWT validation, rate limiting, security middleware
// =============================================================================

pub mod circuit_breaker;
pub mod jwt;
pub mod oauth2;
pub mod rate_limit;
pub mod security;

// Sprint 7-9: Advanced Security
pub mod api_key;
pub mod audit;
pub mod csrf;
pub mod ip_filter;
pub mod rbac;
pub mod session;

pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use jwt::JwtAuth;
pub use rate_limit::RateLimit;
