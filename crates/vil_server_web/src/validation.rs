// =============================================================================
// VIL Server Validation — Valid<T> extractor
// =============================================================================
//
// Valid<T> automatically deserializes JSON and validates using the `validator` crate.
// If validation fails, returns a 422 Unprocessable Entity with detailed error messages.

use axum::extract::{FromRequest, Request};
use serde::de::DeserializeOwned;
use validator::Validate;

use vil_server_core::error::VilError;
use vil_server_core::state::AppState;

/// Extractor that deserializes JSON and validates the payload.
///
/// # Example
/// ```ignore
/// use vil_server_web::Valid;
/// use serde::Deserialize;
/// use validator::Validate;
///
/// #[derive(Deserialize, Validate)]
/// struct CreateUser {
///     #[validate(length(min = 1, max = 100))]
///     name: String,
///     #[validate(email)]
///     email: String,
/// }
///
/// async fn create_user(Valid(req): Valid<CreateUser>) -> impl IntoResponse {
///     // req is guaranteed to be valid here
///     format!("Creating user: {}", req.name)
/// }
/// ```
pub struct Valid<T>(pub T);

#[axum::async_trait]
impl<T> FromRequest<AppState> for Valid<T>
where
    T: DeserializeOwned + Validate + 'static,
{
    type Rejection = VilError;

    async fn from_request(req: Request, state: &AppState) -> Result<Self, Self::Rejection> {
        // First, deserialize
        let axum::Json(value) = axum::Json::<T>::from_request(req, state)
            .await
            .map_err(|e| VilError::bad_request(format!("Invalid JSON: {}", e)))?;

        // Then, validate
        value.validate().map_err(|e| {
            let messages: Vec<String> = e
                .field_errors()
                .iter()
                .flat_map(|(field, errors)| {
                    errors.iter().map(move |err| {
                        format!(
                            "{}: {}",
                            field,
                            err.message
                                .as_ref()
                                .map(|m| m.to_string())
                                .unwrap_or_else(|| format!("{:?}", err.code))
                        )
                    })
                })
                .collect();

            VilError::validation(messages.join("; "))
        })?;

        Ok(Valid(value))
    }
}
