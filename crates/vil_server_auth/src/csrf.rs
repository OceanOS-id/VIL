// =============================================================================
// VIL Server Auth — CSRF Protection Middleware
// =============================================================================
//
// Protects against Cross-Site Request Forgery using the Double-Submit Cookie
// pattern (stateless — no server-side token storage needed).
//
// How it works:
//   1. Server sets a random CSRF token in a cookie (vil-csrf-token)
//   2. Client reads the cookie and sends it back in X-CSRF-Token header
//   3. Server compares cookie value with header value
//   4. If they match → request is legitimate (same origin)
//   5. If they don't match → request is forged (cross-origin can't read cookies)
//
// Safe methods (GET, HEAD, OPTIONS) are exempt from CSRF checks.

use axum::http::{HeaderMap, Method};
use std::collections::HashSet;

/// CSRF protection configuration.
#[derive(Debug, Clone)]
pub struct CsrfConfig {
    /// Cookie name for the CSRF token
    pub cookie_name: String,
    /// Header name to check for the CSRF token
    pub header_name: String,
    /// Token length in bytes (will be hex-encoded)
    pub token_length: usize,
    /// Methods that are exempt from CSRF check
    pub safe_methods: HashSet<Method>,
    /// Paths that are exempt from CSRF check
    pub exempt_paths: Vec<String>,
}

impl Default for CsrfConfig {
    fn default() -> Self {
        let mut safe = HashSet::new();
        safe.insert(Method::GET);
        safe.insert(Method::HEAD);
        safe.insert(Method::OPTIONS);

        Self {
            cookie_name: "vil-csrf-token".to_string(),
            header_name: "x-csrf-token".to_string(),
            token_length: 32,
            safe_methods: safe,
            exempt_paths: Vec::new(),
        }
    }
}

impl CsrfConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a path to the exempt list.
    pub fn exempt_path(mut self, path: impl Into<String>) -> Self {
        self.exempt_paths.push(path.into());
        self
    }
}

/// CSRF token validator.
pub struct CsrfProtection {
    config: CsrfConfig,
}

impl CsrfProtection {
    pub fn new(config: CsrfConfig) -> Self {
        Self { config }
    }

    /// Generate a new CSRF token.
    pub fn generate_token(&self) -> String {
        use std::fmt::Write;
        let mut bytes = vec![0u8; self.config.token_length];
        // Use thread_rng for token generation
        for byte in bytes.iter_mut() {
            *byte = rand_byte();
        }
        let mut hex = String::with_capacity(self.config.token_length * 2);
        for b in &bytes {
            write!(hex, "{:02x}", b).unwrap();
        }
        hex
    }

    /// Check if a request needs CSRF validation.
    pub fn needs_check(&self, method: &Method, path: &str) -> bool {
        // Safe methods are exempt
        if self.config.safe_methods.contains(method) {
            return false;
        }

        // Exempt paths
        for exempt in &self.config.exempt_paths {
            if path.starts_with(exempt) {
                return false;
            }
        }

        true
    }

    /// Validate CSRF token from headers against cookie.
    pub fn validate(&self, headers: &HeaderMap, cookie_token: Option<&str>) -> bool {
        let header_token = headers
            .get(&self.config.header_name)
            .and_then(|v| v.to_str().ok());

        match (header_token, cookie_token) {
            (Some(h), Some(c)) => {
                // Constant-time comparison to prevent timing attacks
                constant_time_eq(h.as_bytes(), c.as_bytes())
            }
            _ => false,
        }
    }

    pub fn config(&self) -> &CsrfConfig {
        &self.config
    }
}

/// Simple pseudo-random byte (not cryptographically secure for production;
/// use `rand` crate for production).
fn rand_byte() -> u8 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos & 0xFF) as u8
}

/// Constant-time byte comparison (prevents timing attacks).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
