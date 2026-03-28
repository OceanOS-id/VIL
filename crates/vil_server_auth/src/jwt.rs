// =============================================================================
// VIL Server Auth — JWT middleware (Tower layer)
// =============================================================================

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use vil_log::app_log;

/// JWT claims structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Expiration time (unix timestamp)
    pub exp: usize,
    /// Issued at (unix timestamp)
    #[serde(default)]
    pub iat: usize,
    /// Roles/permissions
    #[serde(default)]
    pub roles: Vec<String>,
}

/// JWT authentication configuration.
#[derive(Clone)]
pub struct JwtAuth {
    secret: String,
    algorithm: Algorithm,
    required: bool,
}

impl JwtAuth {
    /// Create a new JWT auth configuration with a secret key.
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            algorithm: Algorithm::HS256,
            required: true,
        }
    }

    /// Make JWT optional — if present, validate; if absent, continue.
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// Set the algorithm (default: HS256).
    pub fn algorithm(mut self, alg: Algorithm) -> Self {
        self.algorithm = alg;
        self
    }

    /// Validate a token string and return claims.
    pub fn validate_token(&self, token: &str) -> Result<Claims, JwtError> {
        let validation = Validation::new(self.algorithm);
        let key = DecodingKey::from_secret(self.secret.as_bytes());

        let token_data = decode::<Claims>(token, &key, &validation)
            .map_err(|e| JwtError::InvalidToken(e.to_string()))?;

        Ok(token_data.claims)
    }
}

/// JWT validation errors.
#[derive(Debug)]
pub enum JwtError {
    MissingToken,
    InvalidToken(String),
}

impl std::fmt::Display for JwtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JwtError::MissingToken => write!(f, "Missing authorization token"),
            JwtError::InvalidToken(msg) => write!(f, "Invalid token: {}", msg),
        }
    }
}

/// Axum middleware function for JWT authentication.
/// Use with `axum::middleware::from_fn`.
pub async fn jwt_middleware(request: Request, next: Next) -> Response {
    // Extract JWT config from extensions
    let jwt_auth = request.extensions().get::<JwtAuth>().cloned();

    let jwt_auth = match jwt_auth {
        Some(auth) => auth,
        None => {
            // No JWT config — skip validation
            return next.run(request).await;
        }
    };

    // Extract token from Authorization header
    let token = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    match token {
        Some(token) => {
            match jwt_auth.validate_token(&token) {
                Ok(_claims) => {
                    // Token valid — continue to handler
                    next.run(request).await
                }
                Err(e) => {
                    app_log!(Warn, "jwt_auth", { error: e.to_string() });
                    (
                        StatusCode::UNAUTHORIZED,
                        serde_json::json!({
                            "type": "https://vil.dev/errors/unauthorized",
                            "title": "Unauthorized",
                            "status": 401,
                            "detail": e.to_string()
                        })
                        .to_string(),
                    )
                        .into_response()
                }
            }
        }
        None => {
            if jwt_auth.required {
                (
                    StatusCode::UNAUTHORIZED,
                    serde_json::json!({
                        "type": "https://vil.dev/errors/unauthorized",
                        "title": "Unauthorized",
                        "status": 401,
                        "detail": "Missing Bearer token"
                    })
                    .to_string(),
                )
                    .into_response()
            } else {
                // Optional auth — continue without claims
                next.run(request).await
            }
        }
    }
}
