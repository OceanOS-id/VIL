// =============================================================================
// VIL Server — TLS/HTTPS Enforcement
// =============================================================================
//
// Provides HTTPS redirect and HSTS enforcement middleware.
// When enabled, HTTP requests are redirected to HTTPS.
//
// For TLS termination, use:
//   - Reverse proxy (nginx, envoy, traefik) — recommended for production
//   - axum-server with rustls — for standalone deployment

use axum::http::{HeaderValue, Request};
use axum::middleware::Next;
use axum::response::Response;

/// TLS enforcement configuration.
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Redirect HTTP to HTTPS
    pub redirect_http: bool,
    /// HSTS max-age in seconds (default: 1 year)
    pub hsts_max_age: u64,
    /// Include subdomains in HSTS
    pub hsts_include_subdomains: bool,
    /// HSTS preload flag
    pub hsts_preload: bool,
    /// HTTPS port (default: 443)
    pub https_port: u16,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            redirect_http: true,
            hsts_max_age: 31536000, // 1 year
            hsts_include_subdomains: true,
            hsts_preload: false,
            https_port: 443,
        }
    }
}

impl TlsConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build HSTS header value.
    pub fn hsts_header(&self) -> String {
        let mut val = format!("max-age={}", self.hsts_max_age);
        if self.hsts_include_subdomains {
            val.push_str("; includeSubDomains");
        }
        if self.hsts_preload {
            val.push_str("; preload");
        }
        val
    }
}

/// Middleware that adds HSTS header to all responses.
pub async fn hsts_middleware(request: Request<axum::body::Body>, next: Next) -> Response {
    let config = TlsConfig::default();
    let mut response = next.run(request).await;

    if let Ok(val) = HeaderValue::from_str(&config.hsts_header()) {
        response
            .headers_mut()
            .insert("strict-transport-security", val);
    }

    response
}
