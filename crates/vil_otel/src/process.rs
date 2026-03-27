// =============================================================================
// vil_otel — process.rs
// =============================================================================
//
// Provides a convenience `create` function that initialises the OTel bridge
// (meter provider) from an `OtelConfig` and returns a ready-to-use
// `OtelBridge` handle.
//
// Usage:
// ```ignore
// use vil_otel::process::create;
// use vil_otel::config::OtelConfig;
//
// let bridge = create(OtelConfig::new("my-service")).await?;
// bridge.metrics().record_snapshot(&snap);
// ```
// =============================================================================

use opentelemetry::metrics::MeterProvider as _;
use opentelemetry_sdk::metrics::SdkMeterProvider;

use crate::{
    config::{OtelConfig, OtelProtocol},
    error::OtelFault,
    metrics::MetricsBridge,
    traces::TracesBridge,
};

/// A live OTel bridge handle returned by `create`.
pub struct OtelBridge {
    metrics: MetricsBridge,
    traces:  TracesBridge,
    /// Keeps the meter provider alive for the bridge lifetime.
    _meter_provider: SdkMeterProvider,
}

impl OtelBridge {
    /// Access the metrics bridge.
    pub fn metrics(&self) -> &MetricsBridge {
        &self.metrics
    }

    /// Access the traces bridge.
    pub fn traces(&self) -> &TracesBridge {
        &self.traces
    }
}

/// Initialise the VIL → OTel bridge from config.
///
/// Registers the OTLP exporter, builds meter provider, and returns an
/// `OtelBridge` ready for use inside a VIL service process.
pub async fn create(config: OtelConfig) -> Result<OtelBridge, OtelFault> {
    use opentelemetry_otlp::WithExportConfig as _;

    // Log initialisation attempt via vil_log dict.
    let _init_hash = vil_log::dict::register_str("otel.bridge.init");
    let _svc_hash  = vil_log::dict::register_str(&config.service_name);

    if !config.validate() {
        return Err(OtelFault::InvalidEndpoint);
    }

    // Build the OTLP metrics exporter — protocol branch is cosmetic at this
    // layer; both use tonic since the `tonic` feature is enabled.
    let exporter = match config.protocol {
        OtelProtocol::Grpc | OtelProtocol::Http => {
            opentelemetry_otlp::MetricExporter::builder()
                .with_tonic()
                .with_endpoint(&config.endpoint)
                .build()
                .map_err(|_| OtelFault::InitFailed)?
        }
    };

    // Build the meter provider with a periodic reader.
    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exporter)
        .with_interval(std::time::Duration::from_millis(config.export_interval_ms))
        .build();

    // Build resource with service name + any extra attributes.
    let mut resource_builder = opentelemetry_sdk::Resource::builder_empty()
        .with_service_name(config.service_name.clone());
    for (k, v) in &config.resource_attributes {
        resource_builder = resource_builder
            .with_attribute(opentelemetry::KeyValue::new(k.clone(), v.clone()));
    }
    let resource = resource_builder.build();

    let meter_provider = SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource)
        .build();

    // Obtain a meter scoped to this service.
    // meter() requires &'static str — use a leaked copy for the service name.
    let svc: &'static str = Box::leak(config.service_name.clone().into_boxed_str());
    let meter = meter_provider.meter(svc);

    let metrics = MetricsBridge::with_meter(meter, &config)?;
    let traces  = TracesBridge::new(&config);

    // Register success hash.
    vil_log::dict::register_str("otel.bridge.ready");

    Ok(OtelBridge {
        metrics,
        traces,
        _meter_provider: meter_provider,
    })
}
