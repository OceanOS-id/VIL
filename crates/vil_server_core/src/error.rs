// =============================================================================
// VIL Server Error — RFC 7807 Problem Detail
// =============================================================================

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// Standard error type for vil-server handlers.
/// Automatically maps to RFC 7807 Problem Detail JSON responses.
#[derive(Debug)]
pub struct VilError {
    pub status: StatusCode,
    pub error_type: String,
    pub title: String,
    pub detail: String,
}

#[derive(Serialize)]
struct ProblemDetail {
    r#type: String,
    title: String,
    status: u16,
    detail: String,
}

impl VilError {
    pub fn bad_request(detail: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error_type: "https://vil.dev/errors/bad-request".to_string(),
            title: "Bad Request".to_string(),
            detail: detail.into(),
        }
    }

    pub fn not_found(detail: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            error_type: "https://vil.dev/errors/not-found".to_string(),
            title: "Not Found".to_string(),
            detail: detail.into(),
        }
    }

    pub fn unauthorized(detail: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            error_type: "https://vil.dev/errors/unauthorized".to_string(),
            title: "Unauthorized".to_string(),
            detail: detail.into(),
        }
    }

    pub fn forbidden(detail: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            error_type: "https://vil.dev/errors/forbidden".to_string(),
            title: "Forbidden".to_string(),
            detail: detail.into(),
        }
    }

    pub fn internal(detail: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error_type: "https://vil.dev/errors/internal".to_string(),
            title: "Internal Server Error".to_string(),
            detail: detail.into(),
        }
    }

    pub fn validation(detail: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            error_type: "https://vil.dev/errors/validation".to_string(),
            title: "Validation Error".to_string(),
            detail: detail.into(),
        }
    }

    pub fn rate_limited() -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            error_type: "https://vil.dev/errors/rate-limited".to_string(),
            title: "Too Many Requests".to_string(),
            detail: "Rate limit exceeded. Please retry later.".to_string(),
        }
    }

    pub fn service_unavailable(detail: impl Into<String>) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            error_type: "https://vil.dev/errors/service-unavailable".to_string(),
            title: "Service Unavailable".to_string(),
            detail: detail.into(),
        }
    }
}

impl IntoResponse for VilError {
    fn into_response(self) -> Response {
        let problem = ProblemDetail {
            r#type: self.error_type,
            title: self.title,
            status: self.status.as_u16(),
            detail: self.detail,
        };

        let body = vil_json::to_string(&problem).unwrap_or_else(|_| {
            r#"{"type":"https://vil.dev/errors/internal","title":"Internal Server Error","status":500,"detail":"Failed to serialize error"}"#.to_string()
        });

        (
            self.status,
            [("content-type", "application/problem+json")],
            body,
        )
            .into_response()
    }
}

impl std::fmt::Display for VilError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.title, self.detail)
    }
}

impl std::error::Error for VilError {}

// Allow ? operator with anyhow and common error types
impl From<serde_json::Error> for VilError {
    fn from(err: serde_json::Error) -> Self {
        VilError::bad_request(format!("JSON error: {}", err))
    }
}

impl From<std::io::Error> for VilError {
    fn from(err: std::io::Error) -> Self {
        VilError::internal(format!("I/O error: {}", err))
    }
}
