// =============================================================================
// VIL JWT — Full token lifecycle (sign, verify, refresh)
// =============================================================================

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::Duration;
use vil_server_core::VilError;

/// Token pair returned by sign_pair().
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

/// Full JWT lifecycle manager — sign, verify, refresh.
///
/// # Example
/// ```ignore
/// let jwt = VilJwt::new("my-secret")
///     .access_expiry(Duration::from_secs(900))
///     .refresh_expiry(Duration::from_secs(604800));
///
/// let pair = jwt.sign_pair(&MyClaims { sub: "user123".into(), role: "user".into() })?;
/// let claims: MyClaims = jwt.verify(&pair.access_token)?;
/// let new_access = jwt.refresh::<MyClaims>(&pair.refresh_token)?;
/// ```
#[derive(Clone)]
pub struct VilJwt {
    secret: String,
    access_expiry: Duration,
    refresh_expiry: Duration,
}

impl VilJwt {
    /// Create a new JWT manager with the given secret.
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            access_expiry: Duration::from_secs(900),      // 15 min default
            refresh_expiry: Duration::from_secs(604800),   // 7 days default
        }
    }

    /// Set access token expiry.
    pub fn access_expiry(mut self, dur: Duration) -> Self {
        self.access_expiry = dur;
        self
    }

    /// Set refresh token expiry.
    pub fn refresh_expiry(mut self, dur: Duration) -> Self {
        self.refresh_expiry = dur;
        self
    }

    /// Sign an access + refresh token pair.
    ///
    /// Claims must implement Serialize. The `exp` field is auto-set.
    pub fn sign_pair<T: Serialize + Clone>(&self, claims: &T) -> Result<TokenPair, VilError> {
        let access = self.sign_with_expiry(claims, self.access_expiry, "access")?;
        let refresh = self.sign_with_expiry(claims, self.refresh_expiry, "refresh")?;
        Ok(TokenPair { access_token: access, refresh_token: refresh })
    }

    /// Sign a single access token.
    pub fn sign_access<T: Serialize>(&self, claims: &T) -> Result<String, VilError> {
        self.sign_with_expiry(claims, self.access_expiry, "access")
    }

    /// Verify and decode a token.
    pub fn verify<T: DeserializeOwned>(&self, token: &str) -> Result<T, VilError> {
        let data = decode::<T>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| VilError::unauthorized(format!("invalid token: {e}")))?;
        Ok(data.claims)
    }

    /// Refresh — verify refresh token, issue new access token.
    pub fn refresh<T: Serialize + DeserializeOwned>(&self, refresh_token: &str) -> Result<String, VilError> {
        let claims: T = self.verify(refresh_token)?;
        self.sign_with_expiry(&claims, self.access_expiry, "access")
    }

    /// Get the secret (for advanced use).
    pub fn secret(&self) -> &str {
        &self.secret
    }

    fn sign_with_expiry<T: Serialize>(
        &self,
        claims: &T,
        expiry: Duration,
        _token_type: &str,
    ) -> Result<String, VilError> {
        // Wrap claims with exp
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        #[derive(Serialize)]
        struct Wrapper<'a, T: Serialize> {
            #[serde(flatten)]
            inner: &'a T,
            exp: u64,
            iat: u64,
        }

        let wrapped = Wrapper {
            inner: claims,
            exp: now + expiry.as_secs(),
            iat: now,
        };

        encode(
            &Header::default(),
            &wrapped,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| VilError::internal(format!("jwt sign failed: {e}")))
    }
}
