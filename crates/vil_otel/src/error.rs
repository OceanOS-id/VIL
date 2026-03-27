// =============================================================================
// vil_otel::error — OtelFault
// =============================================================================

use vil_connector_macros::connector_fault;
use vil_log::dict::register_str;

/// Faults that can occur in the VIL OTel bridge.
#[connector_fault]
pub enum OtelFault {
    /// OTLP exporter initialization failed.
    InitFailed,
    /// Invalid endpoint URL.
    InvalidEndpoint,
    /// Pipeline build failed.
    PipelineFailed,
    /// Metrics export error.
    MetricsExportFailed,
    /// Traces export error.
    TracesExportFailed,
}

impl OtelFault {
    /// Returns the registered hash for this fault variant's name.
    pub fn code_hash(&self) -> u32 {
        match self {
            OtelFault::InitFailed          => register_str("otel.fault.init_failed"),
            OtelFault::InvalidEndpoint     => register_str("otel.fault.invalid_endpoint"),
            OtelFault::PipelineFailed      => register_str("otel.fault.pipeline_failed"),
            OtelFault::MetricsExportFailed => register_str("otel.fault.metrics_export_failed"),
            OtelFault::TracesExportFailed  => register_str("otel.fault.traces_export_failed"),
        }
    }
}
