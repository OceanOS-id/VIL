//! # vil_private_rag
//!
//! N08 — Privacy-Preserving RAG: PII redaction before indexing, entity anonymization,
//! and privacy audit logging.

pub mod anonymizer;
pub mod audit;
pub mod config;
pub mod redactor;

pub use anonymizer::Anonymizer;
pub use audit::{AuditEntry, AuditOperation, PrivacyAuditLog};
pub use config::PrivacyConfig;
pub use redactor::{default_pii_patterns, RedactPattern, RedactResult, Redaction, Redactor};

// VIL integration layer
pub mod handlers;
pub mod pipeline_sse;
pub mod plugin;
pub mod vil_semantic;

pub use plugin::PrivateRagPlugin;
pub use vil_semantic::{PrivateRagEvent, PrivateRagFault, PrivateRagState};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_redaction() {
        let redactor = Redactor::new(default_pii_patterns());
        let result = redactor.redact("Contact john@example.com for info.");
        assert!(result.text.contains("[REDACTED_EMAIL]"));
        assert!(!result.text.contains("john@example.com"));
        assert_eq!(result.redactions.len(), 1);
        assert_eq!(result.redactions[0].pattern_name, "email");
    }

    #[test]
    fn test_phone_redaction() {
        let redactor = Redactor::new(default_pii_patterns());
        let result = redactor.redact("Call 555-123-4567 now.");
        assert!(result.text.contains("[REDACTED_PHONE]"));
        assert!(!result.text.contains("555-123-4567"));
    }

    #[test]
    fn test_ssn_redaction() {
        let redactor = Redactor::new(default_pii_patterns());
        let result = redactor.redact("SSN: 123-45-6789");
        assert!(result.text.contains("[REDACTED_SSN]"));
    }

    #[test]
    fn test_credit_card_redaction() {
        let redactor = Redactor::new(default_pii_patterns());
        let result = redactor.redact("Card: 4111 1111 1111 1111");
        assert!(result.text.contains("[REDACTED_CC]"));
    }

    #[test]
    fn test_no_pii_unchanged() {
        let redactor = Redactor::new(default_pii_patterns());
        let input = "This is a normal sentence with no PII.";
        let result = redactor.redact(input);
        assert_eq!(result.text, input);
        assert!(result.redactions.is_empty());
    }

    #[test]
    fn test_anonymization_consistency() {
        let mut anon = Anonymizer::default();
        let first = anon.anonymize("John Doe");
        let second = anon.anonymize("John Doe");
        assert_eq!(first, second, "Same name must produce same pseudonym");
    }

    #[test]
    fn test_anonymization_different_names() {
        let mut anon = Anonymizer::default();
        let a = anon.anonymize("Alice");
        let b = anon.anonymize("Bob");
        assert_ne!(a, b);
    }

    #[test]
    fn test_anonymize_text() {
        let mut anon = Anonymizer::default();
        let entities = vec!["Alice Smith".to_string(), "Bob Jones".to_string()];
        let result = anon.anonymize_text("Alice Smith met Bob Jones at the park.", &entities);
        assert!(!result.contains("Alice Smith"));
        assert!(!result.contains("Bob Jones"));
        assert!(result.contains("PERSON_"));
    }

    #[test]
    fn test_audit_logging() {
        let mut log = PrivacyAuditLog::new();
        assert!(log.is_empty());

        log.log_redaction("email", 2, Some("doc-001"));
        log.log_anonymization(3, Some("doc-001"));

        assert_eq!(log.len(), 2);
        assert!(matches!(
            log.entries[0].operation,
            AuditOperation::Redaction
        ));
        assert!(matches!(
            log.entries[1].operation,
            AuditOperation::Anonymization
        ));
        assert_eq!(log.entries[0].items_affected, 2);
    }

    #[test]
    fn test_audit_json_export() {
        let mut log = PrivacyAuditLog::new();
        log.log_redaction("phone", 1, None);
        let json = log.to_json();
        assert!(json.contains("Redaction"));
        assert!(json.contains("phone"));
    }

    #[test]
    fn test_multiple_pii_in_one_text() {
        let redactor = Redactor::new(default_pii_patterns());
        let result = redactor.redact("Email: a@b.com, SSN: 111-22-3333");
        assert!(result.text.contains("[REDACTED_EMAIL]"));
        assert!(result.text.contains("[REDACTED_SSN]"));
        assert!(result.redactions.len() >= 2);
    }
}
