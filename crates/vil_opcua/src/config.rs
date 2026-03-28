// =============================================================================
// vil_opcua::config — OpcUaConfig
// =============================================================================
//
// Configuration for the OPC-UA client.
// External layout profile acceptable for setup-time data (COMPLIANCE.md §4).
// =============================================================================

/// Security policy for the OPC-UA connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityPolicy {
    /// No security (plaintext). Development / trusted networks only.
    None,
    /// Basic128Rsa15 signing and encryption.
    Basic128Rsa15,
    /// Basic256 signing and encryption.
    Basic256,
    /// Basic256Sha256 signing and encryption (recommended for production).
    Basic256Sha256,
}

impl SecurityPolicy {
    /// Return the OPC-UA security policy URI string.
    pub fn as_uri(&self) -> &'static str {
        match self {
            SecurityPolicy::None => "http://opcfoundation.org/UA/SecurityPolicy#None",
            SecurityPolicy::Basic128Rsa15 => {
                "http://opcfoundation.org/UA/SecurityPolicy#Basic128Rsa15"
            }
            SecurityPolicy::Basic256 => "http://opcfoundation.org/UA/SecurityPolicy#Basic256",
            SecurityPolicy::Basic256Sha256 => {
                "http://opcfoundation.org/UA/SecurityPolicy#Basic256Sha256"
            }
        }
    }
}

/// Configuration for the VIL OPC-UA client.
///
/// # Example YAML
/// ```yaml
/// opcua:
///   endpoint_url: "opc.tcp://plc.factory.local:4840"
///   security_policy: None
///   timeout_ms: 5000
/// ```
#[derive(Debug, Clone)]
pub struct OpcUaConfig {
    /// OPC-UA server endpoint URL (opc.tcp://...).
    pub endpoint_url: String,
    /// Security policy for the session.
    pub security_policy: SecurityPolicy,
    /// Connection and request timeout in milliseconds.
    pub timeout_ms: u64,
    /// Application name reported to the OPC-UA server.
    pub application_name: String,
}

impl OpcUaConfig {
    /// Construct a new `OpcUaConfig` with defaults.
    pub fn new(endpoint_url: impl Into<String>) -> Self {
        Self {
            endpoint_url: endpoint_url.into(),
            security_policy: SecurityPolicy::None,
            timeout_ms: 5_000,
            application_name: String::from("vil_opcua"),
        }
    }

    /// Set the security policy.
    pub fn with_security_policy(mut self, policy: SecurityPolicy) -> Self {
        self.security_policy = policy;
        self
    }

    /// Set the connection timeout.
    pub fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Set the application name reported to the server.
    pub fn with_application_name(mut self, name: impl Into<String>) -> Self {
        self.application_name = name.into();
        self
    }
}
