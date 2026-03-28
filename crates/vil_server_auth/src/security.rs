// =============================================================================
// VIL Server Auth — Security Middleware (OWASP Top 10)
// =============================================================================
//
// Provides security headers and protections against common web vulnerabilities.
// Based on OWASP Top 10 2021:
//   A01: Broken Access Control       → handled by JWT/OAuth2
//   A02: Cryptographic Failures      → TLS enforcement
//   A03: Injection                   → input validation (Valid<T>)
//   A04: Insecure Design             → architecture review
//   A05: Security Misconfiguration   → secure defaults (this module)
//   A06: Vulnerable Components       → cargo audit
//   A07: Auth Failures               → JWT/OAuth2/rate limiting
//   A08: Software Integrity          → SHM capsule isolation
//   A09: Logging Failures            → structured logging
//   A10: SSRF                        → URL validation

use axum::http::{HeaderValue, Request};
use axum::middleware::Next;
use axum::response::Response;

/// Security headers middleware.
///
/// Adds OWASP-recommended security headers to every response:
/// - X-Content-Type-Options: nosniff
/// - X-Frame-Options: DENY
/// - X-XSS-Protection: 0 (modern browsers use CSP instead)
/// - Referrer-Policy: strict-origin-when-cross-origin
/// - Content-Security-Policy: default-src 'self'
/// - Strict-Transport-Security: max-age=31536000 (if HTTPS)
/// - Permissions-Policy: camera=(), microphone=(), geolocation=()
pub async fn security_headers(
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "x-frame-options",
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        "x-xss-protection",
        HeaderValue::from_static("0"),
    );
    headers.insert(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        "content-security-policy",
        HeaderValue::from_static("default-src 'self'"),
    );
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );

    // HSTS — instruct browsers to only use HTTPS for 1 year
    headers.insert(
        "strict-transport-security",
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    response
}

/// Request body size limiter.
///
/// Rejects requests with body larger than max_bytes.
/// Prevents resource exhaustion attacks (DoS via large payloads).
pub struct BodySizeLimit {
    max_bytes: usize,
}

impl BodySizeLimit {
    pub fn new(max_bytes: usize) -> Self {
        Self { max_bytes }
    }

    /// Default: 1MB
    pub fn default_1mb() -> Self {
        Self::new(1024 * 1024)
    }

    /// 10MB limit (for file uploads)
    pub fn large() -> Self {
        Self::new(10 * 1024 * 1024)
    }

    pub fn max_bytes(&self) -> usize {
        self.max_bytes
    }
}

/// IP-based request throttle for brute force protection.
///
/// Tracks failed auth attempts per IP and blocks after threshold.
pub struct BruteForceProtection {
    /// Max failed attempts before blocking
    max_attempts: u64,
    /// Block duration in seconds
    block_duration_secs: u64,
    /// Failed attempts per IP
    attempts: dashmap::DashMap<String, (u64, std::time::Instant)>,
}

impl BruteForceProtection {
    pub fn new(max_attempts: u64, block_duration_secs: u64) -> Self {
        Self {
            max_attempts,
            block_duration_secs,
            attempts: dashmap::DashMap::new(),
        }
    }

    /// Record a failed authentication attempt.
    pub fn record_failure(&self, ip: &str) {
        let mut entry = self.attempts.entry(ip.to_string()).or_insert((0, std::time::Instant::now()));
        entry.0 += 1;
        entry.1 = std::time::Instant::now();
    }

    /// Check if an IP is blocked.
    pub fn is_blocked(&self, ip: &str) -> bool {
        if let Some(entry) = self.attempts.get(ip) {
            let (count, last_attempt) = entry.value();
            if *count >= self.max_attempts {
                if last_attempt.elapsed().as_secs() < self.block_duration_secs {
                    return true;
                }
                // Block expired — reset
                drop(entry);
                self.attempts.remove(ip);
            }
        }
        false
    }

    /// Reset failed attempts for an IP (after successful login).
    pub fn reset(&self, ip: &str) {
        self.attempts.remove(ip);
    }
}

/// OWASP security checklist status.
#[derive(Debug, serde::Serialize)]
pub struct SecurityStatus {
    pub security_headers: bool,
    pub jwt_auth: bool,
    pub rate_limiting: bool,
    pub circuit_breaker: bool,
    pub body_size_limit: bool,
    pub cors_configured: bool,
    pub request_id_propagation: bool,
    pub structured_logging: bool,
    pub graceful_shutdown: bool,
    pub capsule_isolation: bool,
}

impl SecurityStatus {
    /// Default vil-server security posture.
    /// All built-in features are enabled by default.
    pub fn default_posture() -> Self {
        Self {
            security_headers: true,
            jwt_auth: false,  // opt-in
            rate_limiting: false, // opt-in
            circuit_breaker: false, // opt-in
            body_size_limit: true,
            cors_configured: true,
            request_id_propagation: true,
            structured_logging: true,
            graceful_shutdown: true,
            capsule_isolation: true,
        }
    }
}
