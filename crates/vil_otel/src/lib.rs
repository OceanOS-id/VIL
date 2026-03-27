// =============================================================================
// vil_otel — VIL OpenTelemetry Export
// =============================================================================
//
// Bridge vil_obs metrics and vil_log events to OpenTelemetry (OTLP).
//
// Architecture:
//   - `config`  — OtelConfig (endpoint, service_name, protocol, attributes)
//   - `metrics` — MetricsBridge: vil_obs CounterSnapshot → OTel counters
//   - `traces`  — TracesBridge: LogSlotHeader → OTel span attributes
//   - `error`   — OtelFault (plain enum, register_str hashed codes)
//   - `process` — create() initialises the bridge from OtelConfig
//
// Quick start:
// ```rust,ignore
// use vil_otel::{process, config::OtelConfig};
//
// let bridge = process::create(OtelConfig::new("my-service")).await?;
// bridge.metrics().record_snapshot(&snap);
// ```
// =============================================================================

pub mod config;
pub mod error;
pub mod metrics;
pub mod process;
pub mod state;
pub mod traces;

pub use config::{OtelConfig, OtelProtocol};
pub use error::OtelFault;
pub use metrics::{CounterSnapshot, MetricsBridge};
pub use process::OtelBridge;
pub use state::OtelExportState;
pub use traces::{InferredSpanKind, LogSlotHeader, TracesBridge};
