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

// Auth essentials (new in enhance-iter01)
pub mod password;
pub mod jwt_full;
pub mod claims;

pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use jwt::JwtAuth;
pub use rate_limit::RateLimit;
pub use password::VilPassword;
pub use jwt_full::{VilJwt, TokenPair};
pub use claims::VilClaims;
