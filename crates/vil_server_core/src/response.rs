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
    pub fn ok(data: T) -> Self {
        Self {
            status: StatusCode::OK,
            data,
        }
    }

    pub fn created(data: T) -> Self {
        Self {
            status: StatusCode::CREATED,
            data,
        }
    }
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
