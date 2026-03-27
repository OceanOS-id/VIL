// =============================================================================
// VIL Server SHM Response — Zero-copy response writing
// =============================================================================
//
// ShmResponse writes response data into an ExchangeHeap region and streams
// it to the client via hyper. This allows response data to be shared with
// downstream mesh services without additional copies.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde::Serialize;
use std::sync::Arc;

use vil_shm::ExchangeHeap;

/// A response that writes its body to SHM before sending to client.
///
/// Use this when the response data needs to be accessible by other
/// co-located services via the mesh (zero-copy forwarding).
///
/// # Example
/// ```no_run
/// use vil_server_core::shm_response::ShmResponse;
///
/// async fn process() -> ShmResponse {
///     ShmResponse::ok(b"response data")
/// }
/// ```
pub struct ShmResponse {
    status: StatusCode,
    body: Bytes,
    content_type: &'static str,
}

impl ShmResponse {
    /// Create a 200 OK response with raw bytes.
    pub fn ok(body: &[u8]) -> Self {
        Self {
            status: StatusCode::OK,
            body: Bytes::copy_from_slice(body),
            content_type: "application/octet-stream",
        }
    }

    /// Create a 200 OK response with JSON data.
    ///
    /// Uses vil_json (SIMD-accelerated when the "simd" feature is enabled).
    pub fn json<T: Serialize>(data: &T) -> Result<Self, vil_json::JsonError> {
        let bytes = vil_json::to_vec(data)?;
        Ok(Self {
            status: StatusCode::OK,
            body: Bytes::from(bytes),
            content_type: "application/json",
        })
    }

    /// Create a response with custom status code.
    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Write the response body to SHM before returning.
    /// This allows mesh services to read the response data via zero-copy.
    pub fn write_to_shm(self, heap: &Arc<ExchangeHeap>) -> Self {
        if !self.body.is_empty() {
            let region_id = heap.create_region("vil_http_resp", self.body.len() * 2);
            if let Some(offset) = heap.alloc_bytes(region_id, self.body.len(), 8) {
                heap.write_bytes(region_id, offset, &self.body);
                tracing::debug!(
                    len = self.body.len(),
                    "response written to SHM for mesh forwarding"
                );
            }
        }
        self
    }
}

impl IntoResponse for ShmResponse {
    fn into_response(self) -> Response {
        (
            self.status,
            [(axum::http::header::CONTENT_TYPE, self.content_type)],
            self.body,
        )
            .into_response()
    }
}

/// JSON response that is also written to SHM.
///
/// Combines `axum::Json<T>` semantics with SHM write-through.
pub struct ShmJson<T: Serialize> {
    pub data: T,
    pub heap: Option<Arc<ExchangeHeap>>,
}

impl<T: Serialize> ShmJson<T> {
    pub fn new(data: T) -> Self {
        Self { data, heap: None }
    }

    /// Enable SHM write-through for this response.
    pub fn with_shm(mut self, heap: Arc<ExchangeHeap>) -> Self {
        self.heap = Some(heap);
        self
    }
}

impl<T: Serialize> IntoResponse for ShmJson<T> {
    fn into_response(self) -> Response {
        match vil_json::to_vec(&self.data) {
            Ok(bytes) => {
                // Write to SHM if heap is provided
                if let Some(heap) = &self.heap {
                    let region_id = heap.create_region("vil_json_resp", bytes.len() * 2);
                    if let Some(offset) = heap.alloc_bytes(region_id, bytes.len(), 8) {
                        heap.write_bytes(region_id, offset, &bytes);
                    }
                }

                (
                    StatusCode::OK,
                    [(axum::http::header::CONTENT_TYPE, "application/json")],
                    Bytes::from(bytes),
                )
                    .into_response()
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize ShmJson response");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}
