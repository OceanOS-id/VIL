//! VIL Observer Dashboard
//!
//! Provides a lightweight embedded web UI for monitoring VilApp services.
//!
//! Enable with: `VilApp::new("app").observer(true)`
//!
//! Dashboard is served at `/_vil/dashboard/`
//! API is served at `/_vil/api/`

pub mod api;
pub mod metrics;
pub mod dashboard;
pub mod events;
pub mod sidecar;

pub use sidecar::sidecar;

use axum::Router;

/// Create the observer router with all dashboard and API routes.
pub fn observer_router() -> Router {
    Router::new()
        .merge(api::api_routes())
        .merge(dashboard::dashboard_routes())
}
