// =============================================================================
// vil_otel::traces — TracesBridge
// =============================================================================
//
// Converts vil_log LogSlot headers to OpenTelemetry span attributes.
// This is a bridge layer — it does NOT replace the vil_log ring or drains.
//
// Typical flow:
//   1. LogSlot is drained from the ring by a drain task.
//   2. TracesBridge.slot_to_span_attrs() extracts OTel attributes.
//   3. The caller creates an OTel span and sets these attributes.
//
// Usage:
//   let bridge = TracesBridge::new(&config);
//   let attrs = bridge.slot_to_span_attrs(&slot);
// =============================================================================

use opentelemetry::{trace::SpanKind, KeyValue};

use crate::config::OtelConfig;

/// Attribute key constants — registered in vil_log dict for hash lookup.
const ATTR_SERVICE:   &str = "service.name";
const ATTR_LEVEL:     &str = "vil.log.level";
const ATTR_CATEGORY:  &str = "vil.log.category";
const ATTR_PROCESS:   &str = "vil.log.process_id";
const ATTR_TIMESTAMP: &str = "vil.log.timestamp_ns";
const ATTR_VERSION:   &str = "vil.log.version";
const ATTR_SERVICE_H: &str = "vil.log.service_hash";
const ATTR_HANDLER_H: &str = "vil.log.handler_hash";
const ATTR_NODE_H:    &str = "vil.log.node_hash";

/// A lightweight representation of a LogSlot header for OTel bridging.
/// This mirrors `vil_log::VilLogHeader` without a direct dependency.
#[derive(Debug, Clone, Copy, Default)]
pub struct LogSlotHeader {
    pub timestamp_ns:  u64,
    pub level:         u8,
    pub category:      u8,
    pub version:       u8,
    pub service_hash:  u32,
    pub handler_hash:  u32,
    pub node_hash:     u32,
    pub process_id:    u64,
}

/// Span kind inferred from log category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferredSpanKind {
    Server,
    Client,
    Internal,
}

impl From<InferredSpanKind> for SpanKind {
    fn from(k: InferredSpanKind) -> Self {
        match k {
            InferredSpanKind::Server   => SpanKind::Server,
            InferredSpanKind::Client   => SpanKind::Client,
            InferredSpanKind::Internal => SpanKind::Internal,
        }
    }
}

/// Bridge that converts VIL log slot headers to OTel span attributes.
pub struct TracesBridge {
    service_name: String,
}

impl TracesBridge {
    /// Create a new TracesBridge.
    pub fn new(config: &OtelConfig) -> Self {
        // Register attribute key strings in vil_log dict.
        vil_log::dict::register_str(ATTR_SERVICE);
        vil_log::dict::register_str(ATTR_LEVEL);
        vil_log::dict::register_str(ATTR_CATEGORY);
        vil_log::dict::register_str(ATTR_PROCESS);
        vil_log::dict::register_str(ATTR_TIMESTAMP);
        vil_log::dict::register_str(ATTR_VERSION);
        vil_log::dict::register_str(ATTR_SERVICE_H);
        vil_log::dict::register_str(ATTR_HANDLER_H);
        vil_log::dict::register_str(ATTR_NODE_H);

        Self {
            service_name: config.service_name.clone(),
        }
    }

    /// Convert a `LogSlotHeader` to a Vec of OTel `KeyValue` attributes.
    ///
    /// The caller uses these attributes to annotate an OTel span.
    pub fn slot_to_span_attrs(&self, header: &LogSlotHeader) -> Vec<KeyValue> {
        vec![
            KeyValue::new(ATTR_SERVICE,   self.service_name.clone()),
            KeyValue::new(ATTR_LEVEL,     header.level as i64),
            KeyValue::new(ATTR_CATEGORY,  header.category as i64),
            KeyValue::new(ATTR_PROCESS,   header.process_id as i64),
            KeyValue::new(ATTR_TIMESTAMP, header.timestamp_ns as i64),
            KeyValue::new(ATTR_VERSION,   header.version as i64),
            KeyValue::new(ATTR_SERVICE_H, header.service_hash as i64),
            KeyValue::new(ATTR_HANDLER_H, header.handler_hash as i64),
            KeyValue::new(ATTR_NODE_H,    header.node_hash as i64),
        ]
    }

    /// Infer the appropriate OTel span kind from the log category byte.
    ///
    /// Category values mirror `vil_log::LogCategory`:
    ///   0=App, 1=Access, 2=Ai, 3=Db, 4=Mq, 5=System, 6=Security
    pub fn infer_span_kind(&self, category: u8) -> InferredSpanKind {
        match category {
            1 => InferredSpanKind::Server,  // Access — inbound HTTP/gRPC
            3 => InferredSpanKind::Client,  // Db — outbound database calls
            4 => InferredSpanKind::Client,  // Mq — outbound message publish
            _ => InferredSpanKind::Internal,
        }
    }

    /// Map a vil_log level byte to a human-readable string.
    pub fn level_name(level: u8) -> &'static str {
        match level {
            0 => "TRACE",
            1 => "DEBUG",
            2 => "INFO",
            3 => "WARN",
            4 => "ERROR",
            5 => "FATAL",
            _ => "UNKNOWN",
        }
    }

    /// Map a vil_log category byte to a human-readable string.
    pub fn category_name(category: u8) -> &'static str {
        match category {
            0 => "App",
            1 => "Access",
            2 => "Ai",
            3 => "Db",
            4 => "Mq",
            5 => "System",
            6 => "Security",
            _ => "Unknown",
        }
    }
}
