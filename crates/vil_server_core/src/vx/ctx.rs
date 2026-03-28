// =============================================================================
// VX ServiceCtx — Process-aware context for VX endpoints
// =============================================================================
//
// Each VX endpoint handler receives a ServiceCtx that provides:
// - Private heap state (Arc<T>, downcast)
// - Tri-Lane send/trigger/control methods via TriLaneRouter
// - Service identity

use std::any::Any;
use std::sync::Arc;

use axum::extract::{Extension, FromRequestParts};
use axum::http::request::Parts;

use super::tri_lane::{Lane, TriLaneRouter};
use crate::state::AppState;

/// Newtype for injecting service name via Extension layer.
/// Automatically provided by `VilApp::run()` for each registered ServiceProcess.
#[derive(Debug, Clone)]
pub struct ServiceName(pub String);

/// Process-aware context injected into VX endpoint handlers.
///
/// Provides access to the service's private heap state and
/// the Tri-Lane router for inter-service communication.
///
/// # Example
/// ```ignore
/// async fn handle(ctx: &ServiceCtx) -> Result<String, VilError> {
///     let db = ctx.state::<DbPool>()?;
///     ctx.send("analytics", b"page_view").await?;
///     Ok("ok".into())
/// }
/// ```
#[derive(Clone)]
pub struct ServiceCtx {
    /// Name of the owning service process
    service_name: String,
    /// Private heap state (downcast via `state::<T>()`)
    state: Arc<dyn Any + Send + Sync>,
    /// Shared Tri-Lane router for inter-service messaging
    tri_lane: Arc<TriLaneRouter>,
}

impl ServiceCtx {
    /// Create a new service context.
    pub fn new(
        service_name: impl Into<String>,
        state: Arc<dyn Any + Send + Sync>,
        tri_lane: Arc<TriLaneRouter>,
    ) -> Self {
        Self {
            service_name: service_name.into(),
            state,
            tri_lane,
        }
    }

    /// Get the service name.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Downcast the private heap state to a concrete type.
    ///
    /// Returns `Err` if the type does not match.
    pub fn state<T: Send + Sync + 'static>(&self) -> Result<&T, crate::VilError> {
        self.state.downcast_ref::<T>().ok_or_else(|| {
            crate::VilError::internal(format!(
                "Service '{}': state type mismatch (expected {})",
                self.service_name,
                std::any::type_name::<T>(),
            ))
        })
    }

    /// Get a reference to the raw state Arc (for advanced usage).
    pub fn state_raw(&self) -> &Arc<dyn Any + Send + Sync> {
        &self.state
    }

    /// Get a reference to the Tri-Lane router.
    pub fn tri_lane(&self) -> &Arc<TriLaneRouter> {
        &self.tri_lane
    }

    // -------------------------------------------------------------------------
    // Tri-Lane convenience methods
    // -------------------------------------------------------------------------

    /// Send data to a target service via the **Data Lane**.
    ///
    /// Use this for payload transfer (request/response bodies, file data).
    pub async fn send(&self, target: &str, data: &[u8]) -> Result<usize, crate::VilError> {
        self.tri_lane
            .send(&self.service_name, target, Lane::Data, data)
            .await
            .map_err(|e| {
                crate::VilError::internal(format!(
                    "Data Lane send {} -> {}: {}",
                    self.service_name, target, e
                ))
            })
    }

    /// Send data to a target service via the **Trigger Lane**.
    ///
    /// Use this for request initiation, auth tokens, session start signals.
    pub async fn trigger(&self, target: &str, data: &[u8]) -> Result<usize, crate::VilError> {
        self.tri_lane
            .send(&self.service_name, target, Lane::Trigger, data)
            .await
            .map_err(|e| {
                crate::VilError::internal(format!(
                    "Trigger Lane send {} -> {}: {}",
                    self.service_name, target, e
                ))
            })
    }

    /// Send data to a target service via the **Control Lane**.
    ///
    /// Use this for backpressure signals, circuit breaker, health propagation.
    /// Control Lane is never blocked by Data Lane congestion.
    pub async fn control(&self, target: &str, data: &[u8]) -> Result<usize, crate::VilError> {
        self.tri_lane
            .send(&self.service_name, target, Lane::Control, data)
            .await
            .map_err(|e| {
                crate::VilError::internal(format!(
                    "Control Lane send {} -> {}: {}",
                    self.service_name, target, e
                ))
            })
    }
}

impl std::fmt::Debug for ServiceCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceCtx")
            .field("service_name", &self.service_name)
            .field("tri_lane_routes", &self.tri_lane.route_count())
            .finish()
    }
}

// =============================================================================
// Axum Extractor — enables `ctx: ServiceCtx` as handler parameter
// =============================================================================
// Requires VilApp (which injects Extension<Arc<TriLaneRouter>> and
// Extension<ServiceName> per service). Falls back gracefully when used
// with plain VilServer.

#[axum::async_trait]
impl FromRequestParts<AppState> for ServiceCtx {
    type Rejection = crate::VilError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract TriLaneRouter from Extension (injected by VilApp::run)
        let Extension(tri_lane): Extension<Arc<TriLaneRouter>> =
            Extension::from_request_parts(parts, _state)
                .await
                .map_err(|_| {
                    crate::VilError::internal(
                    "ServiceCtx requires VilApp (TriLaneRouter not found in request extensions)"
                )
                })?;

        // Extract service name from Extension (injected by VilApp::run per-service)
        let Extension(svc_name): Extension<ServiceName> =
            Extension::from_request_parts(parts, _state)
                .await
                .map_err(|_| {
                    crate::VilError::internal(
                        "ServiceCtx requires VilApp (ServiceName not found in request extensions)",
                    )
                })?;

        // Extract service-specific state if available
        let state: Arc<dyn Any + Send + Sync> = if let Ok(Extension(s)) =
            Extension::<Arc<dyn Any + Send + Sync>>::from_request_parts(parts, _state).await
        {
            s
        } else {
            Arc::new(()) // No state registered for this service
        };

        Ok(ServiceCtx::new(svc_name.0, state, tri_lane))
    }
}
