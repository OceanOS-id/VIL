// =============================================================================
// VIL Server — OpenTelemetry Integration
// =============================================================================
//
// Provides OpenTelemetry-compatible trace and span export.
// Traces are collected in-process and can be exported via:
//   - OTLP (gRPC/HTTP) to Jaeger, Tempo, etc.
//   - Stdout (for development)
//   - In-memory (for testing)
//
// Key design: vil-server auto-creates spans for every handler,
// mesh hop, and SHM operation — zero-annotation instrumentation.

use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime};


/// Trace ID — 128-bit unique identifier for a distributed trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceId(pub u128);

/// Span ID — 64-bit unique identifier for a span within a trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpanId(pub u64);

impl TraceId {
    pub fn generate() -> Self {
        let hi = std::time::SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u128;
        Self(hi ^ (SPAN_COUNTER.fetch_add(1, Ordering::Relaxed) as u128))
    }

    pub fn to_hex(&self) -> String {
        format!("{:032x}", self.0)
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        u128::from_str_radix(hex, 16).ok().map(Self)
    }
}

impl SpanId {
    pub fn generate() -> Self {
        Self(SPAN_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    pub fn to_hex(&self) -> String {
        format!("{:016x}", self.0)
    }
}

static SPAN_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Span status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SpanStatus {
    Ok,
    Error,
    Unset,
}

/// Span kind (OpenTelemetry-compatible).
#[derive(Debug, Clone, Copy, Serialize)]
pub enum SpanKind {
    Server,
    Client,
    Producer,
    Consumer,
    Internal,
}

/// A completed span record.
#[derive(Debug, Clone, Serialize)]
pub struct SpanRecord {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub kind: SpanKind,
    pub status: SpanStatus,
    pub start_time_unix_ns: u64,
    pub duration_ns: u64,
    pub attributes: Vec<(String, String)>,
    pub service_name: String,
}

/// Active span builder — used during request processing.
pub struct SpanBuilder {
    trace_id: TraceId,
    span_id: SpanId,
    parent_span_id: Option<SpanId>,
    name: String,
    kind: SpanKind,
    start: Instant,
    start_unix_ns: u64,
    attributes: Vec<(String, String)>,
    service_name: String,
}

impl SpanBuilder {
    pub fn new(name: impl Into<String>, kind: SpanKind, service_name: impl Into<String>) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();

        Self {
            trace_id: TraceId::generate(),
            span_id: SpanId::generate(),
            parent_span_id: None,
            name: name.into(),
            kind,
            start: Instant::now(),
            start_unix_ns: now.as_nanos() as u64,
            attributes: Vec::new(),
            service_name: service_name.into(),
        }
    }

    /// Set the parent trace context.
    pub fn with_parent(mut self, trace_id: TraceId, parent_span_id: SpanId) -> Self {
        self.trace_id = trace_id;
        self.parent_span_id = Some(parent_span_id);
        self
    }

    /// Add an attribute.
    pub fn attr(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push((key.into(), value.into()));
        self
    }

    /// Finish the span and record it.
    pub fn finish(self, status: SpanStatus) -> SpanRecord {
        SpanRecord {
            trace_id: self.trace_id.to_hex(),
            span_id: self.span_id.to_hex(),
            parent_span_id: self.parent_span_id.map(|s| s.to_hex()),
            name: self.name,
            kind: self.kind,
            status,
            start_time_unix_ns: self.start_unix_ns,
            duration_ns: self.start.elapsed().as_nanos() as u64,
            attributes: self.attributes,
            service_name: self.service_name,
        }
    }

    pub fn trace_id(&self) -> TraceId {
        self.trace_id
    }

    pub fn span_id(&self) -> SpanId {
        self.span_id
    }
}

/// Span collector — stores completed spans for export.
pub struct SpanCollector {
    spans: Arc<std::sync::RwLock<Vec<SpanRecord>>>,
    max_spans: usize,
    /// Total spans collected (including evicted)
    total_collected: AtomicU64,
}

impl SpanCollector {
    pub fn new(max_spans: usize) -> Self {
        Self {
            spans: Arc::new(std::sync::RwLock::new(Vec::with_capacity(max_spans))),
            max_spans,
            total_collected: AtomicU64::new(0),
        }
    }

    /// Record a completed span.
    pub fn record(&self, span: SpanRecord) {
        self.total_collected.fetch_add(1, Ordering::Relaxed);
        let mut spans = self.spans.write().unwrap();
        if spans.len() >= self.max_spans {
            spans.remove(0); // Ring buffer eviction
        }
        spans.push(span);
    }

    /// Get recent spans for export.
    pub fn drain(&self) -> Vec<SpanRecord> {
        let mut spans = self.spans.write().unwrap();
        std::mem::take(&mut *spans)
    }

    /// Get recent spans without draining.
    pub fn recent(&self, limit: usize) -> Vec<SpanRecord> {
        let spans = self.spans.read().unwrap();
        let start = spans.len().saturating_sub(limit);
        spans[start..].to_vec()
    }

    /// Get total spans collected (including evicted).
    pub fn total_collected(&self) -> u64 {
        self.total_collected.load(Ordering::Relaxed)
    }

    /// Get current buffer size.
    pub fn buffered(&self) -> usize {
        self.spans.read().unwrap().len()
    }
}

impl Default for SpanCollector {
    fn default() -> Self {
        Self::new(10000)
    }
}

// =============================================================================
// W3C Trace Context Propagation
// =============================================================================

/// W3C traceparent header format:
/// `00-{trace_id}-{parent_span_id}-{flags}`
pub struct TraceContext {
    pub trace_id: TraceId,
    pub parent_span_id: SpanId,
    pub sampled: bool,
}

impl TraceContext {
    /// Parse W3C traceparent header.
    pub fn from_header(header: &str) -> Option<Self> {
        let parts: Vec<&str> = header.split('-').collect();
        if parts.len() != 4 || parts[0] != "00" {
            return None;
        }

        let trace_id = TraceId::from_hex(parts[1])?;
        let parent_span_id = u64::from_str_radix(parts[2], 16).ok().map(SpanId)?;
        let flags = u8::from_str_radix(parts[3], 16).unwrap_or(0);

        Some(Self {
            trace_id,
            parent_span_id,
            sampled: flags & 0x01 != 0,
        })
    }

    /// Format as W3C traceparent header.
    pub fn to_header(&self, span_id: SpanId) -> String {
        let flags = if self.sampled { "01" } else { "00" };
        format!("00-{}-{}-{}", self.trace_id.to_hex(), span_id.to_hex(), flags)
    }
}
