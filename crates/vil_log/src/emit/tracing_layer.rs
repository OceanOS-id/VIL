// =============================================================================
// vil_log::emit::tracing_layer — VilTracingLayer
// =============================================================================
//
// Implements tracing_subscriber::Layer.
// On on_event, converts a tracing Event into a LogSlot (category=App)
// and pushes it to the global ring.
// =============================================================================

use std::time::{SystemTime, UNIX_EPOCH};

use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

use crate::dict::register_str;
use crate::emit::ring::try_global_striped;
use crate::types::{LogCategory, LogLevel, LogSlot, VilLogHeader};

/// A `tracing_subscriber::Layer` that forwards tracing events to the VIL log ring.
///
/// Install via:
/// ```rust,ignore
/// use tracing_subscriber::prelude::*;
/// let subscriber = tracing_subscriber::registry()
///     .with(VilTracingLayer::new());
/// tracing::subscriber::set_global_default(subscriber).unwrap();
/// ```
pub struct VilTracingLayer;

impl VilTracingLayer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VilTracingLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Subscriber> Layer<S> for VilTracingLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let striped = match try_global_striped() {
            Some(s) => s,
            None => return,
        };

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let level = tracing_level_to_vil(event.metadata().level());
        let target = event.metadata().target();
        let service_hash = register_str(target);
        let handler_hash = register_str(event.metadata().name());

        let mut slot = LogSlot::default();
        slot.header = VilLogHeader {
            timestamp_ns: ts,
            level: level as u8,
            category: LogCategory::App as u8,
            version: 1,
            service_hash,
            handler_hash,
            node_hash: 0,
            process_id: std::process::id() as u64,
            ..VilLogHeader::default()
        };

        // Collect event fields into the payload as msgpack KV map.
        let mut visitor = FieldCollector::default();
        event.record(&mut visitor);

        if let Ok(encoded) = rmp_serde::to_vec_named(&visitor.fields) {
            let len = encoded.len().min(192);
            slot.payload[..len].copy_from_slice(&encoded[..len]);
        }

        let _ = striped.try_push(slot);
    }
}

fn tracing_level_to_vil(level: &Level) -> LogLevel {
    match *level {
        Level::TRACE => LogLevel::Trace,
        Level::DEBUG => LogLevel::Debug,
        Level::INFO => LogLevel::Info,
        Level::WARN => LogLevel::Warn,
        Level::ERROR => LogLevel::Error,
    }
}

// =============================================================================
// FieldCollector — visits tracing event fields
// =============================================================================

#[derive(Default)]
struct FieldCollector {
    fields: std::collections::BTreeMap<String, serde_json::Value>,
}

impl tracing::field::Visit for FieldCollector {
    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::from(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::from(value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::from(value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::from(value));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::from(value));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::from(format!("{:?}", value)),
        );
    }
}
