// =============================================================================
// VIL Server — Middleware Composition Builder
// =============================================================================
//
// Provides a fluent API for composing middleware stacks.
// Instead of manually layering Tower middleware, use the builder:
//
//   MiddlewareStack::new()
//       .timeout(Duration::from_secs(30))
//       .compression()
//       .security_headers()
//       .request_logging()
//       .build(router)
//
// This ensures correct ordering (outermost layer runs first).

use axum::middleware as axum_mw;
use axum::Router;
use std::time::Duration;

use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tower_http::compression::CompressionLayer;

use crate::state::AppState;
use crate::timeout::TimeoutLayer;

/// Fluent middleware stack builder.
///
/// Middleware are applied in reverse order (last added = outermost).
/// The builder ensures correct ordering for common patterns.
pub struct MiddlewareStack {
    timeout: Option<Duration>,
    compression: bool,
    cors: bool,
    tracing: bool,
    security_headers: bool,
    request_logging: bool,
    handler_metrics: bool,
    request_tracker: bool,
}

impl MiddlewareStack {
    pub fn new() -> Self {
        Self {
            timeout: None,
            compression: false,
            cors: true,         // enabled by default
            tracing: true,      // enabled by default
            security_headers: false,
            request_logging: false,
            handler_metrics: true, // enabled by default
            request_tracker: true, // enabled by default
        }
    }

    /// Add request timeout.
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Enable response compression (gzip).
    pub fn compression(mut self) -> Self {
        self.compression = true;
        self
    }

    /// Enable CORS (default: permissive). Disable with no_cors().
    pub fn no_cors(mut self) -> Self {
        self.cors = false;
        self
    }

    /// Disable trace layer.
    pub fn no_tracing(mut self) -> Self {
        self.tracing = false;
        self
    }

    /// Enable OWASP security headers.
    pub fn security_headers(mut self) -> Self {
        self.security_headers = true;
        self
    }

    /// Enable configurable request logging.
    pub fn request_logging(mut self) -> Self {
        self.request_logging = true;
        self
    }

    /// Disable per-handler metrics.
    pub fn no_handler_metrics(mut self) -> Self {
        self.handler_metrics = false;
        self
    }

    /// Disable request tracker.
    pub fn no_request_tracker(mut self) -> Self {
        self.request_tracker = false;
        self
    }

    /// Apply all configured middleware to a router.
    ///
    /// Ordering (outermost first):
    /// 1. Timeout (if configured)
    /// 2. Compression (if enabled)
    /// 3. Security headers (if enabled)
    /// 4. Handler metrics
    /// 5. Request logging / tracker
    /// 6. CORS
    /// 7. Tracing
    pub fn apply(self, router: Router<AppState>, state: &AppState) -> Router<AppState> {
        let mut app = router;

        // Inner layers first (applied last = runs first after routing)

        if self.tracing {
            app = app.layer(TraceLayer::new_for_http());
        }

        if self.cors {
            app = app.layer(CorsLayer::permissive());
        }

        if self.request_tracker {
            app = app.layer(axum_mw::from_fn_with_state(
                state.clone(),
                crate::middleware::request_tracker,
            ));
        }

        if self.request_logging {
            app = app.layer(axum_mw::from_fn_with_state(
                state.clone(),
                crate::request_log::request_logger,
            ));
        }

        if self.handler_metrics {
            app = app.layer(axum_mw::from_fn_with_state(
                state.clone(),
                crate::obs_middleware::handler_metrics,
            ));
        }

        if self.compression {
            app = app.layer(CompressionLayer::new());
        }

        // Timeout is outermost
        if let Some(timeout) = self.timeout {
            app = app.layer(TimeoutLayer::new(timeout));
        }

        app
    }
}

impl Default for MiddlewareStack {
    fn default() -> Self {
        Self::new()
    }
}
