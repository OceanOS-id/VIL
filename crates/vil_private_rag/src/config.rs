//! Configuration for privacy-preserving RAG.

use serde::{Deserialize, Serialize};

/// Privacy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// Whether to enable PII redaction.
    pub enable_redaction: bool,
    /// Whether to enable entity anonymization.
    pub enable_anonymization: bool,
    /// Whether to log all privacy operations to an audit log.
    pub enable_audit_log: bool,
    /// Pseudonym prefix for anonymization.
    pub anonymization_prefix: String,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            enable_redaction: true,
            enable_anonymization: true,
            enable_audit_log: true,
            anonymization_prefix: "PERSON".into(),
        }
    }
}
