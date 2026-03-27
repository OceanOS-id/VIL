// =============================================================================
// VIL Server Middleware — Request tracking (optimized)
// =============================================================================

use axum::extract::State;
use axum::http::{HeaderValue, Request};
use axum::middleware::Next;
use axum::response::Response;
use std::time::Instant;

use crate::state::AppState;

/// Optimized request tracking middleware.
///
/// Optimizations vs original:
/// - Reuse existing X-Request-Id header value (avoid UUID generation when possible)
/// - Use `method.as_str()` instead of `.clone().to_string()`
/// - Use `itoa` for integer formatting (stack-allocated)
/// - Avoid `format!()` for X-Response-Time header
pub async fn request_tracker(
    State(state): State<AppState>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let start = Instant::now();

    // Reuse existing request ID or generate new one
    let has_id = request.headers().contains_key("x-request-id");
    if !has_id {
        // Only generate UUID when not provided (saves ~0.2µs when header exists)
        let id = uuid::Uuid::new_v4().to_string();
        if let Ok(val) = HeaderValue::from_str(&id) {
            request.headers_mut().insert("x-request-id", val);
        }
    }

    // Track request start (atomic increment only)
    state.request_start();

    // Execute the handler
    let mut response = next.run(request).await;

    // Track request end
    let duration_ms = start.elapsed().as_millis() as u64;
    state.request_end(duration_ms);

    if response.status().is_server_error() {
        state.route_error();
    }

    // Inject response time header (stack-allocated formatting)
    let mut buf = itoa::Buffer::new();
    let duration_str = buf.format(duration_ms);
    // Build "Nms" without format! macro
    let mut time_header = String::with_capacity(duration_str.len() + 2);
    time_header.push_str(duration_str);
    time_header.push_str("ms");
    if let Ok(val) = HeaderValue::from_str(&time_header) {
        response.headers_mut().insert("x-response-time", val);
    }

    response
}
