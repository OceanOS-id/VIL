// =============================================================================
// VIL Server Response — Standard response wrappers
// =============================================================================

use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;
use std::sync::Arc;
use vil_shm::ExchangeHeap;

/// Wrapper for successful JSON responses with standard envelope.
pub struct VilResponse<T: Serialize> {
    pub status: StatusCode,
    pub data: T,
}

impl<T: Serialize> VilResponse<T> {
    /// 200 OK — standard success response.
    pub fn ok(data: T) -> Self {
        Self { status: StatusCode::OK, data }
    }

    /// 201 Created — resource created successfully.
    pub fn created(data: T) -> Self {
        Self { status: StatusCode::CREATED, data }
    }

    /// 202 Accepted — request accepted for async processing.
    pub fn accepted(data: T) -> Self {
        Self { status: StatusCode::ACCEPTED, data }
    }

    /// Custom HTTP status code with typed data.
    pub fn with_status(data: T, status: StatusCode) -> Self {
        Self { status, data }
    }
}

/// Error response helpers — return JSON error body without typed data.
impl VilResponse<serde_json::Value> {
    /// 400 Bad Request
    pub fn bad_request(detail: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_REQUEST, data: _err_body("BAD_REQUEST", detail) }
    }

    /// 401 Unauthorized
    pub fn unauthorized(detail: impl Into<String>) -> Self {
        Self { status: StatusCode::UNAUTHORIZED, data: _err_body("UNAUTHORIZED", detail) }
    }

    /// 403 Forbidden
    pub fn forbidden(detail: impl Into<String>) -> Self {
        Self { status: StatusCode::FORBIDDEN, data: _err_body("FORBIDDEN", detail) }
    }

    /// 404 Not Found
    pub fn not_found(detail: impl Into<String>) -> Self {
        Self { status: StatusCode::NOT_FOUND, data: _err_body("NOT_FOUND", detail) }
    }

    /// 409 Conflict
    pub fn conflict(detail: impl Into<String>) -> Self {
        Self { status: StatusCode::CONFLICT, data: _err_body("CONFLICT", detail) }
    }

    /// 422 Unprocessable Entity
    pub fn unprocessable(detail: impl Into<String>) -> Self {
        Self { status: StatusCode::UNPROCESSABLE_ENTITY, data: _err_body("VALIDATION_ERROR", detail) }
    }

    /// 429 Too Many Requests
    pub fn too_many(detail: impl Into<String>) -> Self {
        Self { status: StatusCode::TOO_MANY_REQUESTS, data: _err_body("RATE_LIMITED", detail) }
    }

    /// 500 Internal Server Error
    pub fn internal_error(detail: impl Into<String>) -> Self {
        Self { status: StatusCode::INTERNAL_SERVER_ERROR, data: _err_body("INTERNAL_ERROR", detail) }
    }

    /// 503 Service Unavailable
    pub fn unavailable(detail: impl Into<String>) -> Self {
        Self { status: StatusCode::SERVICE_UNAVAILABLE, data: _err_body("UNAVAILABLE", detail) }
    }
}

fn _err_body(code: &str, message: impl Into<String>) -> serde_json::Value {
    serde_json::json!({ "ok": false, "error": { "code": code, "message": message.into() } })
}

impl<T: Serialize> VilResponse<T> {
    /// Enable SHM write-through — response data is also written to ExchangeHeap
    /// for zero-copy mesh forwarding.
    pub fn with_shm(self, heap: Arc<ExchangeHeap>) -> ShmVilResponse<T> {
        ShmVilResponse { inner: self, heap }
    }
}

impl<T: Serialize> IntoResponse for VilResponse<T> {
    fn into_response(self) -> axum::response::Response {
        // Use vil_json (SIMD-accelerated) instead of serde_json
        match vil_json::to_vec(&self.data) {
            Ok(bytes) => (
                self.status,
                [(axum::http::header::CONTENT_TYPE, "application/json")],
                bytes::Bytes::from(bytes),
            )
                .into_response(),
            Err(_) => (self.status, axum::Json(self.data)).into_response(),
        }
    }
}

/// VilResponse with automatic SHM write-through.
/// Response data is written to ExchangeHeap for zero-copy mesh forwarding.
pub struct ShmVilResponse<T: Serialize> {
    inner: VilResponse<T>,
    heap: Arc<ExchangeHeap>,
}

impl<T: Serialize> IntoResponse for ShmVilResponse<T> {
    fn into_response(self) -> axum::response::Response {
        match vil_json::to_vec(&self.inner.data) {
            Ok(bytes) => {
                // Write to SHM for mesh forwarding
                let region_id = self.heap.create_region("vil_response", bytes.len() * 2);
                if let Some(offset) = self.heap.alloc_bytes(region_id, bytes.len(), 8) {
                    self.heap.write_bytes(region_id, offset, &bytes);
                }
                (
                    self.inner.status,
                    [(axum::http::header::CONTENT_TYPE, "application/json")],
                    bytes::Bytes::from(bytes),
                )
                    .into_response()
            }
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

/// Empty success response (204 No Content).
pub struct NoContent;

impl IntoResponse for NoContent {
    fn into_response(self) -> axum::response::Response {
        StatusCode::NO_CONTENT.into_response()
    }
}
