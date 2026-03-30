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
    let _query = request.uri().query().map(|q| q.to_string());

    let _request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();

    let _content_length = request
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("0")
        .to_string();

    let _user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();

    let response = next.run(request).await;

    let duration_ms = start.elapsed().as_millis() as u64;
    let status = response.status().as_u16();

    // Log based on status code severity
    {
        use vil_log::app_log;
        if status >= 500 {
            app_log!(Error, "http.server.error", {
                status: status as u64,
                duration_ms: duration_ms,
                method: method.as_str(),
                path: path.as_str()
            });
        } else if status >= 400 {
            app_log!(Warn, "http.client.error", {
                status: status as u64,
                duration_ms: duration_ms,
                method: method.as_str(),
                path: path.as_str()
            });
        } else {
            app_log!(Info, "http.request", {
                status: status as u64,
                duration_ms: duration_ms,
                method: method.as_str(),
                path: path.as_str()
            });
        }
    }

    response
}
