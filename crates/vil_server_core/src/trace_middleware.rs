// =============================================================================
// VIL Server — Distributed Tracing Middleware (Optimized)
// =============================================================================
//
// Auto-creates spans for HTTP requests with configurable sampling.
// Propagates W3C Trace Context between services.
//
// Optimization: sampling reduces overhead for high-throughput services.
// Incoming requests with traceparent are ALWAYS traced (propagation).
// Requests without traceparent are sampled at TRACE_SAMPLE_RATE.

use axum::extract::State;
use axum::http::{HeaderValue, Request};
use axum::middleware::Next;
use axum::response::Response;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::otel::*;
use crate::state::AppState;

/// Trace sampling counter.
static TRACE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Trace sample rate: 1 = every request (default), N = every Nth request.
/// Incoming requests with traceparent header are always traced regardless.
pub static TRACE_SAMPLE_RATE: AtomicU64 = AtomicU64::new(1);

/// Distributed tracing middleware (optimized with sampling).
pub async fn tracing_middleware(
    State(state): State<AppState>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    // Check for incoming trace context (always honor propagation)
    let parent_ctx = request
        .headers()
        .get("traceparent")
        .and_then(|v| v.to_str().ok())
        .and_then(TraceContext::from_header);

    let has_parent = parent_ctx.is_some();

    // Sampling: skip trace creation if not sampled AND no parent context
    let sample_rate = TRACE_SAMPLE_RATE.load(Ordering::Relaxed);
    let counter = TRACE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let should_trace = has_parent || sample_rate <= 1 || counter % sample_rate == 0;

    if !should_trace {
        return next.run(request).await;
    }

    // Build span (optimized: avoid .to_string() allocations)
    let method = request.method().as_str();
    let path = request.uri().path();
    let service_name = state.name();

    let mut span_name = String::with_capacity(method.len() + 1 + path.len());
    span_name.push_str(method);
    span_name.push(' ');
    span_name.push_str(path);

    let mut span = SpanBuilder::new(span_name, SpanKind::Server, service_name)
        .attr("http.method", method)
        .attr("http.target", path);

    if let Some(ctx) = &parent_ctx {
        span = span.with_parent(ctx.trace_id, ctx.parent_span_id);
    }

    let trace_id = span.trace_id();
    let span_id = span.span_id();

    // Inject trace context
    let traceparent = TraceContext {
        trace_id,
        parent_span_id: span_id,
        sampled: true,
    }
    .to_header(span_id);

    if let Ok(val) = HeaderValue::from_str(&traceparent) {
        request.headers_mut().insert("traceparent", val);
    }

    let response = next.run(request).await;

    let status_code = response.status().as_u16();
    let span_status = if status_code >= 500 {
        SpanStatus::Error
    } else {
        SpanStatus::Ok
    };

    // Finish span (optimized: use stack-allocated status string)
    let mut status_buf = itoa::Buffer::new();
    let record = span
        .attr("http.status_code", status_buf.format(status_code))
        .finish(span_status);

    state.span_collector().record(record);

    let mut response = response;
    if let Ok(val) = HeaderValue::from_str(&traceparent) {
        response.headers_mut().insert("traceparent", val);
    }

    response
}
