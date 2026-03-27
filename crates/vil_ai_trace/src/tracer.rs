//! AiTracer — collect trace spans with RAII guards.

use crate::metrics::AiMetrics;
use crate::span::{AiOperation, SpanStatus, TraceSpan};
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// AI tracer that collects trace spans.
pub struct AiTracer {
    spans: DashMap<String, TraceSpan>,
    counter: AtomicU64,
}

impl AiTracer {
    pub fn new() -> Self {
        Self {
            spans: DashMap::new(),
            counter: AtomicU64::new(0),
        }
    }

    /// Generate a unique span ID.
    fn next_span_id(&self) -> String {
        let id = self.counter.fetch_add(1, Ordering::Relaxed);
        format!("span-{id:08x}")
    }

    /// Start a new span, returning an RAII guard that ends it on drop.
    pub fn start_span(
        self: &Arc<Self>,
        trace_id: &str,
        operation: AiOperation,
        parent_id: Option<&str>,
    ) -> SpanGuard {
        let span_id = self.next_span_id();
        let span = TraceSpan {
            trace_id: trace_id.to_string(),
            span_id: span_id.clone(),
            parent_id: parent_id.map(|s| s.to_string()),
            operation,
            start_ms: current_time_ms(),
            end_ms: None,
            attributes: HashMap::new(),
            status: SpanStatus::Running,
        };
        self.spans.insert(span_id.clone(), span);

        SpanGuard {
            tracer: Arc::clone(self),
            span_id,
        }
    }

    /// Record a completed span directly.
    pub fn record_span(&self, span: TraceSpan) {
        self.spans.insert(span.span_id.clone(), span);
    }

    /// Export all spans as JSON.
    pub fn export_json(&self) -> String {
        let spans: Vec<TraceSpan> = self.spans.iter().map(|r| r.value().clone()).collect();
        crate::exporter::export_json(&spans)
    }

    /// Get all spans.
    pub fn all_spans(&self) -> Vec<TraceSpan> {
        self.spans.iter().map(|r| r.value().clone()).collect()
    }

    /// Aggregate metrics from all spans.
    pub fn metrics(&self) -> AiMetrics {
        let mut m = AiMetrics::default();
        let mut latencies = Vec::new();
        let mut cache_hits = 0u64;
        let mut cache_checks = 0u64;

        for entry in self.spans.iter() {
            let span = entry.value();
            m.total_spans += 1;

            if span.operation == AiOperation::LlmCall {
                m.total_llm_calls += 1;
            }

            if span.status == SpanStatus::Error {
                m.error_count += 1;
            }

            if let Some(dur) = span.duration_ms() {
                latencies.push(dur as f64);
            }

            if let Some(tokens) = span.attributes.get("tokens") {
                if let Ok(t) = tokens.parse::<u64>() {
                    m.total_tokens += t;
                }
            }
            if let Some(cost) = span.attributes.get("cost") {
                if let Ok(c) = cost.parse::<f64>() {
                    m.total_cost += c;
                }
            }
            if let Some(hit) = span.attributes.get("cache_hit") {
                cache_checks += 1;
                if hit == "true" {
                    cache_hits += 1;
                }
            }
        }

        if !latencies.is_empty() {
            m.avg_latency_ms = latencies.iter().sum::<f64>() / latencies.len() as f64;
        }
        if cache_checks > 0 {
            m.cache_hit_rate = cache_hits as f64 / cache_checks as f64;
        }

        m
    }

    /// Number of recorded spans.
    pub fn span_count(&self) -> usize {
        self.spans.len()
    }
}

impl Default for AiTracer {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard that ends a span on drop.
pub struct SpanGuard {
    tracer: Arc<AiTracer>,
    span_id: String,
}

impl SpanGuard {
    /// Get the span ID.
    pub fn span_id(&self) -> &str {
        &self.span_id
    }

    /// Set an attribute on the span.
    pub fn set_attribute(&self, key: &str, value: &str) {
        if let Some(mut span) = self.tracer.spans.get_mut(&self.span_id) {
            span.attributes.insert(key.to_string(), value.to_string());
        }
    }

    /// Mark the span as error.
    pub fn set_error(&self) {
        if let Some(mut span) = self.tracer.spans.get_mut(&self.span_id) {
            span.status = SpanStatus::Error;
        }
    }

    /// Explicitly end the span.
    pub fn end(self) {
        // Drop triggers the end.
    }
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        if let Some(mut span) = self.tracer.spans.get_mut(&self.span_id) {
            span.end_ms = Some(current_time_ms());
            if span.status == SpanStatus::Running {
                span.status = SpanStatus::Ok;
            }
        }
    }
}

fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
