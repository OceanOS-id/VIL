// =============================================================================
// VIL Server Auth — API Key Authentication Middleware
// =============================================================================
//
// Validates API keys from request headers or query parameters.
// Supports multiple keys with optional scoping (per-service, per-route).
//
// Header modes:
//   X-API-Key: <key>
//   Authorization: ApiKey <key>
//   Authorization: Bearer <key> (with api_key mode)
//
// Query mode:
//   ?api_key=<key>

use axum::extract::Request;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use std::sync::Arc;

/// API key store and validator.
#[derive(Clone)]
pub struct ApiKeyAuth {
    /// Map of api_key → ApiKeyInfo
    keys: Arc<DashMap<String, ApiKeyInfo>>,
    /// Header name to check (default: "x-api-key")
    header_name: String,
    /// Also check query parameter
    check_query: bool,
}

/// Information associated with an API key.
#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    /// Key owner/label
    pub name: String,
    /// Allowed scopes (empty = unrestricted)
    pub scopes: Vec<String>,
    /// Whether this key is active
    pub active: bool,
}

impl ApiKeyAuth {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(DashMap::new()),
            header_name: "x-api-key".to_string(),
            check_query: false,
        }
    }

    /// Set custom header name.
    pub fn header(mut self, name: impl Into<String>) -> Self {
        self.header_name = name.into();
        self
    }

    /// Also accept API key from query parameter `?api_key=`.
    pub fn allow_query(mut self) -> Self {
        self.check_query = true;
        self
    }

    /// Register an API key.
    pub fn add_key(&self, key: impl Into<String>, name: impl Into<String>) {
        self.keys.insert(key.into(), ApiKeyInfo {
            name: name.into(),
            scopes: Vec::new(),
            active: true,
        });
    }

    /// Register an API key with scopes.
    pub fn add_key_scoped(
        &self,
        key: impl Into<String>,
        name: impl Into<String>,
        scopes: Vec<String>,
    ) {
        self.keys.insert(key.into(), ApiKeyInfo {
            name: name.into(),
            scopes,
            active: true,
        });
    }

    /// Revoke an API key.
    pub fn revoke_key(&self, key: &str) {
        if let Some(mut entry) = self.keys.get_mut(key) {
            entry.active = false;
        }
    }

    /// Validate a key. Returns the key info if valid.
    pub fn validate(&self, key: &str) -> Option<ApiKeyInfo> {
        self.keys.get(key).and_then(|info| {
            if info.active {
                Some(info.clone())
            } else {
                None
            }
        })
    }

    /// Extract API key from request headers or query string.
    pub fn extract_key(&self, headers: &HeaderMap, uri: &axum::http::Uri) -> Option<String> {
        // Check custom header
        if let Some(val) = headers.get(&self.header_name) {
            if let Ok(key) = val.to_str() {
                return Some(key.to_string());
            }
        }

        // Check Authorization: ApiKey <key>
        if let Some(auth) = headers.get("authorization") {
            if let Ok(val) = auth.to_str() {
                if let Some(key) = val.strip_prefix("ApiKey ") {
                    return Some(key.to_string());
                }
            }
        }

        // Check query parameter
        if self.check_query {
            if let Some(query) = uri.query() {
                for pair in query.split('&') {
                    if let Some(key) = pair.strip_prefix("api_key=") {
                        return Some(key.to_string());
                    }
                }
            }
        }

        None
    }

    /// Get number of registered keys.
    pub fn key_count(&self) -> usize {
        self.keys.len()
    }
}

impl Default for ApiKeyAuth {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware function for API key validation.
///
/// Add to router:
///   .layer(axum::middleware::from_fn(api_key_middleware))
///
/// Note: Requires ApiKeyAuth to be stored in request extensions.
pub async fn api_key_middleware(
    request: Request,
    next: Next,
) -> Response {
    // Check for API key in extensions
    let auth = request.extensions().get::<ApiKeyAuth>().cloned();

    if let Some(auth) = auth {
        let key = auth.extract_key(request.headers(), request.uri());

        match key {
            Some(k) => {
                if auth.validate(&k).is_none() {
                    return (
                        StatusCode::UNAUTHORIZED,
                        axum::Json(serde_json::json!({
                            "error": "Invalid API key",
                            "status": 401,
                        })),
                    ).into_response();
                }
            }
            None => {
                return (
                    StatusCode::UNAUTHORIZED,
                    axum::Json(serde_json::json!({
                        "error": "API key required",
                        "hint": "Provide via X-API-Key header or Authorization: ApiKey <key>",
                        "status": 401,
                    })),
                ).into_response();
            }
        }
    }

    next.run(request).await
}
