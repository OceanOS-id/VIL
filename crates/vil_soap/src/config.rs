// =============================================================================
// vil_soap::config — SoapConfig
// =============================================================================
//
// Configuration for the SOAP/WSDL client.
// External layout profile is acceptable for setup-time data (COMPLIANCE.md §4).
// =============================================================================

/// Configuration for the VIL SOAP/WSDL client.
///
/// # Example YAML
/// ```yaml
/// soap:
///   wsdl_url: "http://service.example.com/api?wsdl"
///   endpoint: "http://service.example.com/api"
///   timeout_ms: 30000
/// ```
#[derive(Debug, Clone)]
pub struct SoapConfig {
    /// URL to the WSDL descriptor (used for service discovery).
    pub wsdl_url: String,
    /// Actual endpoint URL where SOAP requests are sent.
    pub endpoint: String,
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
}

impl SoapConfig {
    /// Construct a new `SoapConfig`.
    pub fn new(wsdl_url: impl Into<String>, endpoint: impl Into<String>) -> Self {
        Self {
            wsdl_url: wsdl_url.into(),
            endpoint: endpoint.into(),
            timeout_ms: 30_000,
        }
    }

    /// Override the request timeout.
    pub fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }
}
