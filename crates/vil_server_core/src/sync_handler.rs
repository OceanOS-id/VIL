// =============================================================================
// VIL Server Sync Handler — auto spawn_blocking for CPU-bound handlers
// =============================================================================
//
// Wraps synchronous (blocking) functions into Axum-compatible async handlers.
// CPU-bound work (ML inference, image processing, crypto) should NOT run on
// the Tokio executor. This module provides `blocking()` to auto-dispatch
// sync functions to Tokio's blocking thread pool.
//
// Usage:
//   .route("/predict", post(blocking(predict)))
//
// The wrapped function runs on spawn_blocking, freeing the async executor
// for I/O-bound tasks.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::future::Future;
use std::pin::Pin;

/// Wrap a synchronous (blocking) function as an Axum handler.
///
/// The function will be dispatched to Tokio's blocking thread pool via
/// `tokio::task::spawn_blocking`. This prevents CPU-bound work from
/// starving the async executor.
///
/// # Example
/// ```no_run
/// use vil_server_core::sync_handler::blocking;
///
/// // CPU-bound handler — runs on blocking thread pool
/// fn predict(body: Vec<u8>) -> String {
///     // Heavy computation...
///     format!("Processed {} bytes", body.len())
/// }
///
/// // Use in route:
/// // .route("/predict", post(blocking(predict)))
/// ```
pub fn blocking<F, R>(f: F) -> impl Fn() -> Pin<Box<dyn Future<Output = Response> + Send>> + Clone
where
    F: Fn() -> R + Send + Sync + Clone + 'static,
    R: IntoResponse + Send + 'static,
{
    move || {
        let f = f.clone();
        Box::pin(async move {
            match tokio::task::spawn_blocking(move || f().into_response()).await {
                Ok(response) => response,
                Err(e) => {
                    {
                        use vil_log::app_log;
                        app_log!(Error, "handler.blocking.panic", { error: e.to_string() });
                    }
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        })
    }
}

/// Wrap a synchronous function that takes extracted data as an Axum handler.
///
/// # Example
/// ```no_run
/// use vil_server_core::sync_handler::blocking_with;
/// use bytes::Bytes;
///
/// fn process(data: Vec<u8>) -> String {
///     format!("Processed {} bytes", data.len())
/// }
///
/// // The async wrapper extracts Bytes, then dispatches to blocking
/// async fn handler(body: Bytes) -> impl axum::response::IntoResponse {
///     blocking_with(move || process(body.to_vec())).await
/// }
/// ```
pub async fn blocking_with<F, R>(f: F) -> Response
where
    F: FnOnce() -> R + Send + 'static,
    R: IntoResponse + Send + 'static,
{
    match tokio::task::spawn_blocking(f).await {
        Ok(response) => response.into_response(),
        Err(e) => {
            {
                use vil_log::app_log;
                app_log!(Error, "handler.blocking.panic", { error: e.to_string() });
            }
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
