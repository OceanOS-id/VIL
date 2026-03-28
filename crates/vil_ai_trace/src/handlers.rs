//! HTTP handlers for the AI trace plugin — wired to real AiTracer state.

use std::sync::Arc;
use vil_server::prelude::*;

use crate::span::TraceSpan;
use crate::tracer::AiTracer;

// ── Response types ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct TraceStatsBody {
    pub span_count: usize,
    pub total_llm_calls: u64,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub avg_latency_ms: f64,
    pub error_count: u64,
    pub recent_spans: Vec<TraceSpan>,
}

// ── Handlers ────────────────────────────────────────────────────────

/// GET /stats — return real span count and recent spans from the tracer.
pub async fn stats_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<TraceStatsBody>> {
    let tracer = ctx.state::<Arc<AiTracer>>().expect("AiTracer");
    let metrics = tracer.metrics();
    let all_spans = tracer.all_spans();

    // Return up to the 20 most recent spans.
    let recent_spans: Vec<TraceSpan> = {
        let mut spans = all_spans;
        spans.sort_by(|a, b| b.start_ms.cmp(&a.start_ms));
        spans.truncate(20);
        spans
    };

    Ok(VilResponse::ok(TraceStatsBody {
        span_count: tracer.span_count(),
        total_llm_calls: metrics.total_llm_calls,
        total_tokens: metrics.total_tokens,
        total_cost: metrics.total_cost,
        avg_latency_ms: metrics.avg_latency_ms,
        error_count: metrics.error_count,
        recent_spans,
    }))
}
