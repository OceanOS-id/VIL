// =============================================================================
// VIL Server SHM Extractor — Zero-copy request body via ExchangeHeap
// =============================================================================
//
// ShmSlice extracts the request body and writes it into a VIL SHM region.
// Downstream handlers and services can then access it via zero-copy.
//
// Flow:
//   HTTP request body (Bytes)
//     → write to ExchangeHeap region (1 copy)
//     → ShmSlice holds region_id + offset + len
//     → handler reads directly from SHM (0 copy)
//     → can be forwarded to mesh services (0 copy)

use axum::body::Bytes;
use axum::extract::{FromRequest, Request};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::sync::Arc;

use vil_shm::ExchangeHeap;
use vil_types::RegionId;

use crate::state::AppState;

/// Zero-copy request body extractor backed by VIL SHM.
///
/// Writes the incoming request body into an ExchangeHeap region and provides
/// direct access to the data without further copying.
///
/// # Example
/// ```no_run
/// use vil_server_core::shm_extractor::ShmSlice;
///
/// async fn ingest(body: ShmSlice) -> String {
///     format!("Received {} bytes in SHM region", body.len())
/// }
/// ```
pub struct ShmSlice {
    /// The raw bytes (backed by SHM)
    data: Bytes,
    /// SHM region where data is stored
    region_id: RegionId,
    /// Offset within the region
    offset: vil_shm::Offset,
    /// Reference to the heap for downstream forwarding
    heap: Arc<ExchangeHeap>,
}

impl ShmSlice {
    /// Get the data as a byte slice (zero-copy read from SHM)
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get the data length
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the SHM region ID (for forwarding to mesh services)
    pub fn region_id(&self) -> RegionId {
        self.region_id
    }

    /// Get the offset within the SHM region
    pub fn offset(&self) -> vil_shm::Offset {
        self.offset
    }

    /// Get a reference to the underlying ExchangeHeap
    pub fn heap(&self) -> &Arc<ExchangeHeap> {
        &self.heap
    }

    /// Convert to Bytes (cheap clone, reference counted)
    pub fn into_bytes(self) -> Bytes {
        self.data
    }

    /// Try to deserialize the body as JSON.
    ///
    /// Uses vil_json (SIMD-accelerated when the "simd" feature is enabled).
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, vil_json::JsonError> {
        vil_json::from_slice(&self.data)
    }

    /// Get the body as a UTF-8 string
    pub fn text(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.data)
    }
}

impl std::fmt::Debug for ShmSlice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShmSlice")
            .field("len", &self.data.len())
            .field("region_id", &self.region_id)
            .finish()
    }
}

impl std::ops::Deref for ShmSlice {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.data
    }
}

/// Rejection type when ShmSlice extraction fails.
pub enum ShmSliceRejection {
    BodyReadFailed(String),
    ShmWriteFailed(String),
}

impl IntoResponse for ShmSliceRejection {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            ShmSliceRejection::BodyReadFailed(e) => {
                (StatusCode::BAD_REQUEST, format!("Failed to read body: {}", e))
            }
            ShmSliceRejection::ShmWriteFailed(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("SHM write failed: {}", e))
            }
        };
        (status, msg).into_response()
    }
}

#[axum::async_trait]
impl FromRequest<AppState> for ShmSlice {
    type Rejection = ShmSliceRejection;

    async fn from_request(req: Request, state: &AppState) -> Result<Self, Self::Rejection> {
        // Extract body bytes
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|e| ShmSliceRejection::BodyReadFailed(e.to_string()))?;

        let pool = state.shm_pool();
        let heap = pool.heap().clone();

        if bytes.is_empty() {
            return Ok(ShmSlice {
                data: bytes,
                region_id: pool.region_id(),
                offset: vil_shm::Offset::ZERO,
                heap,
            });
        }

        // Write body into pre-allocated SHM pool (single copy, no region creation)
        let (region_id, offset) = pool.alloc_and_write(&bytes)
            .ok_or_else(|| ShmSliceRejection::ShmWriteFailed("Pool allocation failed".into()))?;

        Ok(ShmSlice {
            data: bytes,
            region_id,
            offset,
            heap,
        })
    }
}
