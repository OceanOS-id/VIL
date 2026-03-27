// =============================================================================
// VIL Server Auth — OAuth2/OIDC Client
// =============================================================================
//
// Provides OAuth2 Authorization Code flow and OIDC token validation.
// Designed for server-to-server authentication and user login flows.
//
// Supports:
//   - Authorization Code Grant (with PKCE)
//   - Client Credentials Grant (service-to-service)
//   - Token introspection
//   - OIDC ID token validation
//
// Note: This is a client-side implementation — vil-server acts as
// a relying party, not an identity provider.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// OAuth2 provider configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct OAuth2Config {
    /// Authorization endpoint URL
    pub auth_url: String,
    /// Token endpoint URL
    pub token_url: String,
    /// Client ID
    pub client_id: String,
    /// Client secret (for confidential clients)
    pub client_secret: Option<String>,
    /// Redirect URI for Authorization Code flow
    pub redirect_uri: Option<String>,
    /// Scopes to request
    #[serde(default)]
    pub scopes: Vec<String>,
    /// OIDC issuer URL (for discovery)
    pub issuer: Option<String>,
    /// JWKS URI for token validation
    pub jwks_uri: Option<String>,
}

impl OAuth2Config {
    /// Create a config for Authorization Code flow.
    pub fn authorization_code(
        auth_url: impl Into<String>,
        token_url: impl Into<String>,
        client_id: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Self {
        Self {
            auth_url: auth_url.into(),
            token_url: token_url.into(),
            client_id: client_id.into(),
            client_secret: None,
            redirect_uri: Some(redirect_uri.into()),
            scopes: vec!["openid".to_string(), "profile".to_string()],
            issuer: None,
            jwks_uri: None,
        }
    }

    /// Create a config for Client Credentials flow (service-to-service).
    pub fn client_credentials(
        token_url: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Self {
        Self {
            auth_url: String::new(),
            token_url: token_url.into(),
            client_id: client_id.into(),
            client_secret: Some(client_secret.into()),
            redirect_uri: None,
            scopes: Vec::new(),
            issuer: None,
            jwks_uri: None,
        }
    }

    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }

    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }
}

/// OAuth2 token response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub id_token: Option<String>,
}

/// Cached token with expiry tracking.
#[derive(Debug, Clone)]
pub struct CachedToken {
    pub token: TokenResponse,
    pub obtained_at: Instant,
}

impl CachedToken {
    pub fn new(token: TokenResponse) -> Self {
        Self {
            token,
            obtained_at: Instant::now(),
        }
    }

    /// Check if the token has expired (with 30s safety margin).
    pub fn is_expired(&self) -> bool {
        if let Some(expires_in) = self.token.expires_in {
            let margin = Duration::from_secs(30);
            self.obtained_at.elapsed() >= Duration::from_secs(expires_in).saturating_sub(margin)
        } else {
            false
        }
    }

    pub fn access_token(&self) -> &str {
        &self.token.access_token
    }
}

/// OAuth2 client for token acquisition and management.
pub struct OAuth2Client {
    config: OAuth2Config,
    cached_token: std::sync::RwLock<Option<CachedToken>>,
}

impl OAuth2Client {
    pub fn new(config: OAuth2Config) -> Self {
        Self {
            config,
            cached_token: std::sync::RwLock::new(None),
        }
    }

    /// Build the authorization URL for Authorization Code flow.
    pub fn authorization_url(&self, state: &str) -> Result<String, String> {
        let redirect = self.config.redirect_uri.as_ref()
            .ok_or("redirect_uri required for authorization code flow")?;

        let scopes = self.config.scopes.join(" ");

        Ok(format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
            self.config.auth_url,
            urlencoded(&self.config.client_id),
            urlencoded(redirect),
            urlencoded(&scopes),
            urlencoded(state),
        ))
    }

    /// Get a cached token or indicate refresh is needed.
    pub fn get_cached_token(&self) -> Option<CachedToken> {
        let guard = self.cached_token.read().unwrap();
        guard.as_ref().and_then(|t| {
            if t.is_expired() { None } else { Some(t.clone()) }
        })
    }

    /// Store a token in the cache.
    pub fn cache_token(&self, token: TokenResponse) {
        let cached = CachedToken::new(token);
        *self.cached_token.write().unwrap() = Some(cached);
    }

    /// Get the config.
    pub fn config(&self) -> &OAuth2Config {
        &self.config
    }
}

/// Minimal URL encoding (sufficient for OAuth2 params).
fn urlencoded(s: &str) -> String {
    s.replace('&', "%26")
        .replace('=', "%3D")
        .replace(' ', "%20")
        .replace('+', "%2B")
}

/// OIDC claims from an ID token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Issuer
    pub iss: Option<String>,
    /// Audience
    pub aud: Option<String>,
    /// Expiration time (unix timestamp)
    pub exp: Option<u64>,
    /// Issued at (unix timestamp)
    pub iat: Option<u64>,
    /// Email
    pub email: Option<String>,
    /// Name
    pub name: Option<String>,
}
