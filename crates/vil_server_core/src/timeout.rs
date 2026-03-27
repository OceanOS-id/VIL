// =============================================================================
// VIL Server — Request Timeout Middleware
// =============================================================================
//
// Enforces a maximum request processing duration. If a handler exceeds
// the timeout, the request is aborted with 408 Request Timeout.
//
// Usage:
//   VilServer::new("app")
//       .layer(TimeoutLayer::new(Duration::from_secs(30)))

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tower::{Layer, Service};

/// Layer that applies a request timeout.
#[derive(Clone)]
pub struct TimeoutLayer {
    timeout: Duration,
}

impl TimeoutLayer {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    pub fn from_secs(secs: u64) -> Self {
        Self::new(Duration::from_secs(secs))
    }
}

impl<S> Layer<S> for TimeoutLayer {
    type Service = TimeoutService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TimeoutService {
            inner,
            timeout: self.timeout,
        }
    }
}

/// Service wrapper that enforces request timeout.
#[derive(Clone)]
pub struct TimeoutService<S> {
    inner: S,
    timeout: Duration,
}

impl<S, ReqBody> Service<axum::http::Request<ReqBody>> for TimeoutService<S>
where
    S: Service<axum::http::Request<ReqBody>, Response = Response> + Clone + Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: axum::http::Request<ReqBody>) -> Self::Future {
        let mut inner = self.inner.clone();
        let timeout = self.timeout;

        Box::pin(async move {
            match tokio::time::timeout(timeout, inner.call(req)).await {
                Ok(result) => result,
                Err(_) => {
                    tracing::warn!(
                        timeout_ms = timeout.as_millis() as u64,
                        "request timed out"
                    );
                    Ok(timeout_response(timeout))
                }
            }
        })
    }
}

fn timeout_response(duration: Duration) -> Response {
    let body = serde_json::json!({
        "error": "Request Timeout",
        "message": format!("Request exceeded {}ms timeout", duration.as_millis()),
        "status": 408,
    });
    (
        StatusCode::REQUEST_TIMEOUT,
        axum::Json(body),
    ).into_response()
}
