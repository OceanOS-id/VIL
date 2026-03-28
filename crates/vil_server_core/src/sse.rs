// =============================================================================
// VIL Server SSE — Server-Sent Events support
// =============================================================================
//
// Provides helpers for streaming SSE responses from vil-server handlers.
// Built on Axum's SSE support with VIL integrations.
//
// # Example
// ```no_run
// use vil_server_core::sse::{sse_stream, SseEvent};
//
// async fn events() -> impl IntoResponse {
//     let stream = async_stream::stream! {
//         for i in 0..10 {
//             yield SseEvent::data(format!("message {}", i));
//             tokio::time::sleep(Duration::from_millis(100)).await;
//         }
//     };
//     sse_stream(stream)
// }
// ```

use axum::response::sse::{Event, KeepAlive, Sse};
use futures_core::Stream;
use std::convert::Infallible;

/// A convenience wrapper around Axum's SSE Event.
pub struct SseEvent;

impl SseEvent {
    /// Create an SSE event with data.
    pub fn data(data: impl Into<String>) -> Result<Event, Infallible> {
        Ok(Event::default().data(data.into()))
    }

    /// Create an SSE event with data and event type.
    pub fn named(event_type: &str, data: impl Into<String>) -> Result<Event, Infallible> {
        Ok(Event::default().event(event_type).data(data.into()))
    }

    /// Create an SSE event with JSON data.
    ///
    /// Uses vil_json (SIMD-accelerated when the "simd" feature is enabled).
    pub fn json<T: serde::Serialize>(data: &T) -> Result<Event, Infallible> {
        let json = vil_json::to_string(data).unwrap_or_default();
        Ok(Event::default().data(json))
    }

    /// Create a named SSE event with JSON data.
    ///
    /// Uses vil_json (SIMD-accelerated when the "simd" feature is enabled).
    pub fn named_json<T: serde::Serialize>(
        event_type: &str,
        data: &T,
    ) -> Result<Event, Infallible> {
        let json = vil_json::to_string(data).unwrap_or_default();
        Ok(Event::default().event(event_type).data(json))
    }
}

/// Create an SSE response from a stream of events.
///
/// Automatically adds keep-alive pings every 15 seconds.
pub fn sse_stream<S>(stream: S) -> Sse<S>
where
    S: Stream<Item = Result<Event, Infallible>> + Send + 'static,
{
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Create an SSE response with custom keep-alive interval.
pub fn sse_stream_with_keepalive<S>(stream: S, interval_secs: u64) -> Sse<S>
where
    S: Stream<Item = Result<Event, Infallible>> + Send + 'static,
{
    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(interval_secs)))
}
