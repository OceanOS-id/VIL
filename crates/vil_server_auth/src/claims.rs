// =============================================================================
// VIL Claims — Auto-extract JWT claims from Authorization header
// =============================================================================

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use vil_server_core::VilError;

use crate::jwt_full::VilJwt;

/// Auto-extract JWT claims from `Authorization: Bearer <token>` header.
///
/// Requires `VilJwt` injected via `VilApp::jwt()` or `Extension<Arc<VilJwt>>`.
///
/// # Example
/// ```ignore
/// #[vil_handler]
/// async fn protected(VilClaims(claims): VilClaims<MyClaims>) -> VilResult<String> {
///     Ok(VilResponse::ok(format!("Hello {}", claims.sub)))
/// }
/// ```
pub struct VilClaims<T>(pub T);

#[axum::async_trait]
impl<T, S> FromRequestParts<S> for VilClaims<T>
where
    T: DeserializeOwned + Send + 'static,
    S: Send + Sync,
{
    type Rejection = VilError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Get Authorization header
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| VilError::unauthorized("Missing Authorization header"))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| VilError::unauthorized("Invalid Authorization format"))?;

        // Get VilJwt from Extension
        let jwt = parts
            .extensions
            .get::<Arc<VilJwt>>()
            .ok_or_else(|| {
                VilError::internal(
                    "VilJwt not configured. Add .jwt(VilJwt::new(secret)) to VilApp"
                )
            })?;

        let claims: T = jwt.verify(token)?;
        Ok(VilClaims(claims))
    }
}
