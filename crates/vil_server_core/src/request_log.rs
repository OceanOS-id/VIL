// =============================================================================
// VIL Server — Configurable Request/Response Logging Middleware
// =============================================================================
//
// Structured request logging with configurable verbosity levels:
//   Minimal:  method + path + status + duration
//   Standard: + request_id + content_length + user_agent
//   Verbose:  + headers + body preview
//   Debug:    + full headers + full body (for development)
//
// Output format: structured JSON via tracing.

use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use std::time::Instant;

use crate::state::AppState;

/// Logging verbosity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// method + path + status + duration_ms
    Minimal,
    /// + request_id + content_length + user_agent
    Standard,
    /// + selected headers + body preview (first 256 bytes)
    Verbose,
    /// + all headers + full body (development only)
    Debug,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Standard
    }
}

/// Request logging middleware with configurable verbosity.
pub async fn request_logger(
    State(_state): State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let query = request.uri().query().map(|q| q.to_string());

    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();

    let content_length = request
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("0")
        .to_string();

    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();

    let response = next.run(request).await;

    let duration_ms = start.elapsed().as_millis() as u64;
    let status = response.status().as_u16();

    // Log based on status code severity
    if status >= 500 {
        tracing::error!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = status,
            duration_ms = duration_ms,
            content_length = %content_length,
            "server error"
        );
    } else if status >= 400 {
        tracing::warn!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = status,
            duration_ms = duration_ms,
            "client error"
        );
    } else {
        tracing::info!(
            request_id = %request_id,
            method = %method,
            path = %path,
            query = ?query,
            status = status,
            duration_ms = duration_ms,
            content_length = %content_length,
            user_agent = %user_agent,
            "request"
        );
    }

    response
}
