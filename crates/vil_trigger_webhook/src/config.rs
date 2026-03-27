// =============================================================================
// vil_trigger_webhook::config — WebhookConfig
// =============================================================================
//
// Configuration for the HTTP webhook receiver trigger.
// =============================================================================

/// Configuration for the VIL webhook trigger.
///
/// # Example YAML
/// ```yaml
/// webhook:
///   listen_addr: "0.0.0.0:8090"
///   secret: "my-hmac-secret"
///   path: "/webhook"
/// ```
#[derive(Debug, Clone)]
pub struct WebhookConfig {
    /// Socket address to bind the HTTP listener (e.g. `"0.0.0.0:8090"`).
    pub listen_addr: String,
    /// HMAC-SHA256 secret for signature verification (raw bytes as UTF-8).
    pub secret: String,
    /// HTTP path to receive webhook POST requests (e.g. `"/webhook"`).
    pub path: String,
}

impl WebhookConfig {
    /// Construct a new `WebhookConfig`.
    pub fn new(
        listen_addr: impl Into<String>,
        secret: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        Self {
            listen_addr: listen_addr.into(),
            secret: secret.into(),
            path: path.into(),
        }
    }
}
