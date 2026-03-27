// =============================================================================
// vil_otel::config — OtelConfig
// =============================================================================

use std::collections::HashMap;

/// Transport protocol for OTLP export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OtelProtocol {
    /// gRPC transport (default port 4317).
    Grpc,
    /// HTTP/protobuf transport (default port 4318).
    Http,
}

impl Default for OtelProtocol {
    fn default() -> Self {
        OtelProtocol::Grpc
    }
}

impl std::fmt::Display for OtelProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OtelProtocol::Grpc => write!(f, "grpc"),
            OtelProtocol::Http => write!(f, "http"),
        }
    }
}

/// Configuration for the VIL OpenTelemetry bridge.
#[derive(Debug, Clone)]
pub struct OtelConfig {
    /// OTLP endpoint URL.
    /// - gRPC: "http://localhost:4317"
    /// - HTTP: "http://localhost:4318"
    pub endpoint: String,
    /// Service name reported to the OTel backend.
    pub service_name: String,
    /// Transport protocol (Grpc or Http).
    pub protocol: OtelProtocol,
    /// Additional resource attributes appended to every export.
    pub resource_attributes: HashMap<String, String>,
    /// Export interval in milliseconds. Default: 5000.
    pub export_interval_ms: u64,
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:4317".to_string(),
            service_name: "vil-service".to_string(),
            protocol: OtelProtocol::Grpc,
            resource_attributes: HashMap::new(),
            export_interval_ms: 5_000,
        }
    }
}

impl OtelConfig {
    /// Create a new config with sensible defaults.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            ..Default::default()
        }
    }

    /// Override the endpoint.
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    /// Override the protocol.
    pub fn with_protocol(mut self, protocol: OtelProtocol) -> Self {
        self.protocol = protocol;
        self
    }

    /// Add a resource attribute.
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.resource_attributes.insert(key.into(), value.into());
        self
    }

    /// Override the export interval.
    pub fn with_export_interval_ms(mut self, ms: u64) -> Self {
        self.export_interval_ms = ms;
        self
    }

    /// Validate the configuration.
    pub fn validate(&self) -> bool {
        !self.endpoint.is_empty() && !self.service_name.is_empty()
    }
}
