//! # vil_ai_trace
//!
//! N11 — AI Observability + Tracing: collect trace spans for AI operations,
//! compute metrics, export as JSON.

pub mod exporter;
pub mod metrics;
pub mod span;
pub mod tracer;

pub use metrics::AiMetrics;
pub use span::{AiOperation, SpanStatus, TraceSpan};
pub use tracer::{AiTracer, SpanGuard};

// VIL integration layer
pub mod vil_semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use plugin::AiTracePlugin;
pub use vil_semantic::{TraceEvent, TraceFault, TraceState};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn test_span_creation() {
        let tracer = Arc::new(AiTracer::new());
        let guard = tracer.start_span("trace-1", AiOperation::LlmCall, None);
        assert!(!guard.span_id().is_empty());
        drop(guard);
        assert_eq!(tracer.span_count(), 1);
    }

    #[test]
    fn test_parent_child_spans() {
        let tracer = Arc::new(AiTracer::new());
        let parent = tracer.start_span("t1", AiOperation::AgentStep, None);
        let parent_id = parent.span_id().to_string();
        let child = tracer.start_span("t1", AiOperation::LlmCall, Some(&parent_id));
        let child_id = child.span_id().to_string();
        drop(child);
        drop(parent);

        let spans = tracer.all_spans();
        let child_span = spans.iter().find(|s| s.span_id == child_id).unwrap();
        assert_eq!(child_span.parent_id.as_deref(), Some(parent_id.as_str()));
    }

    #[test]
    fn test_export_json() {
        let tracer = Arc::new(AiTracer::new());
        let guard = tracer.start_span("t1", AiOperation::Embedding, None);
        drop(guard);
        let json = tracer.export_json();
        assert!(json.contains("Embedding"));
        assert!(json.contains("t1"));
    }

    #[test]
    fn test_metrics_aggregation() {
        let tracer = Arc::new(AiTracer::new());

        // Record a completed span with attributes.
        let mut span = TraceSpan {
            trace_id: "t1".into(),
            span_id: "s1".into(),
            parent_id: None,
            operation: AiOperation::LlmCall,
            start_ms: 1000,
            end_ms: Some(1100),
            attributes: HashMap::new(),
            status: SpanStatus::Ok,
        };
        span.attributes.insert("tokens".into(), "500".into());
        span.attributes.insert("cost".into(), "0.05".into());
        tracer.record_span(span);

        let m = tracer.metrics();
        assert_eq!(m.total_llm_calls, 1);
        assert_eq!(m.total_tokens, 500);
        assert!((m.total_cost - 0.05).abs() < 0.001);
        assert!((m.avg_latency_ms - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_concurrent_spans() {
        let tracer = Arc::new(AiTracer::new());
        let mut guards = Vec::new();
        for i in 0..10 {
            guards.push(tracer.start_span(&format!("t-{i}"), AiOperation::ToolCall, None));
        }
        drop(guards);
        assert_eq!(tracer.span_count(), 10);
    }

    #[test]
    fn test_operation_types() {
        let ops = [
            AiOperation::LlmCall,
            AiOperation::Embedding,
            AiOperation::Retrieval,
            AiOperation::Rerank,
            AiOperation::ToolCall,
            AiOperation::AgentStep,
        ];
        for op in &ops {
            assert!(!format!("{op}").is_empty());
        }
    }

    #[test]
    fn test_span_attributes() {
        let tracer = Arc::new(AiTracer::new());
        let guard = tracer.start_span("t1", AiOperation::LlmCall, None);
        guard.set_attribute("model", "gpt-4");
        guard.set_attribute("tokens", "100");
        let sid = guard.span_id().to_string();
        drop(guard);

        let spans = tracer.all_spans();
        let span = spans.iter().find(|s| s.span_id == sid).unwrap();
        assert_eq!(span.attributes.get("model").unwrap(), "gpt-4");
    }

    #[test]
    fn test_span_error_status() {
        let tracer = Arc::new(AiTracer::new());
        let guard = tracer.start_span("t1", AiOperation::LlmCall, None);
        guard.set_error();
        let sid = guard.span_id().to_string();
        drop(guard);

        let spans = tracer.all_spans();
        let span = spans.iter().find(|s| s.span_id == sid).unwrap();
        assert_eq!(span.status, SpanStatus::Error);
        let m = tracer.metrics();
        assert_eq!(m.error_count, 1);
    }
}
