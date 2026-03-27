// =============================================================================
// VIL Server Extractors — RequestId, ShmContext, TriLaneCtx
// =============================================================================

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::HeaderValue;
use std::sync::Arc;

use vil_shm::{ExchangeHeap, RegionStats};

use crate::state::AppState;

/// Unique request identifier, propagated via X-Request-Id header.
/// If no header is present, a new UUID is generated.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[axum::async_trait]
impl FromRequestParts<AppState> for RequestId {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let id = parts
            .headers
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // Ensure the request ID is in the headers for downstream propagation
        parts.headers.insert(
            "x-request-id",
            HeaderValue::from_str(&id).unwrap_or_else(|_| HeaderValue::from_static("unknown")),
        );

        Ok(RequestId(id))
    }
}

/// Context for zero-copy SHM data passing.
///
/// Provides access to VIL's ExchangeHeap for zero-copy request/response handling.
/// When SHM is available, request bodies can be written directly to shared memory
/// regions, enabling zero-copy processing across the handler pipeline.
///
/// # Example
/// ```no_run
/// use vil_server_core::extractors::ShmContext;
///
/// async fn ingest(shm: ShmContext, body: bytes::Bytes) -> &'static str {
///     if shm.available {
///         // Write body to SHM for zero-copy downstream processing
///         tracing::info!("SHM available with {} regions", shm.region_count());
///     }
///     "ok"
/// }
/// ```
#[derive(Clone)]
pub struct ShmContext {
    /// Whether SHM is available on this system
    pub available: bool,
    /// Reference to the shared ExchangeHeap
    heap: Arc<ExchangeHeap>,
}

impl std::fmt::Debug for ShmContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShmContext")
            .field("available", &self.available)
            .field("regions", &self.heap.region_count())
            .finish()
    }
}

impl ShmContext {
    pub fn new(heap: Arc<ExchangeHeap>) -> Self {
        let available = std::path::Path::new("/dev/shm").exists();
        Self { available, heap }
    }

    /// Get region statistics from the ExchangeHeap
    pub fn region_stats(&self) -> Vec<RegionStats> {
        self.heap.all_stats()
    }

    /// Get the number of active regions
    pub fn region_count(&self) -> usize {
        self.heap.region_count()
    }

    /// Get a reference to the underlying ExchangeHeap
    pub fn heap(&self) -> &ExchangeHeap {
        &self.heap
    }
}

#[axum::async_trait]
impl FromRequestParts<AppState> for ShmContext {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(ShmContext::new(state.shm().clone()))
    }
}

/// Tri-Lane context for inter-service communication.
/// Provides access to Trigger, Data, and Control lanes for the current service.
#[derive(Debug, Clone)]
pub struct TriLaneCtx {
    /// Service name this context belongs to
    pub service: String,
}

impl TriLaneCtx {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }
}
