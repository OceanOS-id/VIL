//! VIL Content Guardrails Engine (H07).
//!
//! Provides PII detection, toxicity scoring, and custom allow/deny/redact rules
//! for content moderation before or after LLM processing.
//!
//! ```
//! use vil_guardrails::GuardrailsEngine;
//!
//! let engine = GuardrailsEngine::new();
//! let result = engine.check("Hello, this is safe text.");
//! assert!(result.passed);
//! ```

pub mod config;
pub mod engine;
pub mod pii;
pub mod rules;
pub mod toxicity;

pub use config::{GuardrailsConfig, GuardrailsConfigBuilder};
pub use engine::{GuardrailResult, GuardrailsEngine, Violation};
pub use pii::{PiiDetector, PiiMatch, PiiType};
pub use rules::{Rule, RuleAction, RuleEngine, RuleMatch};
pub use toxicity::ToxicityChecker;

// VIL integration layer
pub mod semantic;
pub mod pipeline_sse;
pub mod handlers;
pub mod plugin;

pub use plugin::GuardrailsPlugin;
pub use semantic::{GuardrailCheckEvent, GuardrailFault, GuardrailsState};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_detection() {
        let detector = PiiDetector::new();
        let matches = detector.detect("Contact me at john@example.com please.");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pii_type, PiiType::Email);
        assert_eq!(matches[0].value, "john@example.com");
    }

    #[test]
    fn test_phone_detection() {
        let detector = PiiDetector::new();
        let matches = detector.detect("Call me at 555-123-4567.");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pii_type, PiiType::Phone);
    }

    #[test]
    fn test_credit_card_detection() {
        let detector = PiiDetector::new();
        let matches = detector.detect("My card is 4111-1111-1111-1111.");
        assert!(matches.iter().any(|m| m.pii_type == PiiType::CreditCard));
    }

    #[test]
    fn test_ssn_detection() {
        let detector = PiiDetector::new();
        let matches = detector.detect("SSN: 123-45-6789");
        assert!(matches.iter().any(|m| m.pii_type == PiiType::SSN));
    }

    #[test]
    fn test_ip_address_detection() {
        let detector = PiiDetector::new();
        let matches = detector.detect("Server IP is 192.168.1.100.");
        assert!(matches.iter().any(|m| m.pii_type == PiiType::IpAddress));
    }

    #[test]
    fn test_toxicity_scoring_clean() {
        let checker = ToxicityChecker::with_defaults();
        let score = checker.score("Hello, how are you today?");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_toxicity_scoring_toxic() {
        let checker = ToxicityChecker::with_defaults();
        let score = checker.score("I hate this and there is violence and abuse here");
        assert!(score > 0.0);
    }

    #[test]
    fn test_custom_rules_deny() {
        let mut engine = RuleEngine::new();
        engine
            .add_rule(Rule {
                name: "no_secrets".to_string(),
                pattern: r"(?i)api[_-]?key".to_string(),
                action: RuleAction::Deny,
            })
            .unwrap();
        assert!(engine.has_deny("My API_KEY is abc123"));
        assert!(!engine.has_deny("No secrets here"));
    }

    #[test]
    fn test_custom_rules_allow() {
        let mut engine = RuleEngine::new();
        engine
            .add_rule(Rule {
                name: "allow_test".to_string(),
                pattern: r"(?i)test_mode".to_string(),
                action: RuleAction::Allow,
            })
            .unwrap();
        assert!(engine.has_allow("Running in test_mode"));
    }

    #[test]
    fn test_custom_rules_redact() {
        let mut engine = RuleEngine::new();
        engine
            .add_rule(Rule {
                name: "redact_password".to_string(),
                pattern: r"password:\s*\S+".to_string(),
                action: RuleAction::Redact,
            })
            .unwrap();
        let redacted = engine.redact("My password: secret123 is here");
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("secret123"));
    }

    #[test]
    fn test_clean_text_passes() {
        let engine = GuardrailsEngine::new();
        let result = engine.check("This is a perfectly clean and safe message.");
        assert!(result.passed);
        assert!(result.violations.is_empty());
        assert!(result.pii_found.is_empty());
        assert_eq!(result.toxicity_score, 0.0);
    }

    #[test]
    fn test_multiple_pii_in_one_text() {
        let detector = PiiDetector::new();
        let matches = detector.detect(
            "Email: user@test.com, Phone: 555-123-4567, IP: 10.0.0.1",
        );
        let types: Vec<&PiiType> = matches.iter().map(|m| &m.pii_type).collect();
        assert!(types.contains(&&PiiType::Email));
        assert!(types.contains(&&PiiType::Phone));
        assert!(types.contains(&&PiiType::IpAddress));
    }

    #[test]
    fn test_config_builder() {
        let config = GuardrailsConfigBuilder::new()
            .pii_enabled(false)
            .toxicity_threshold(0.8)
            .build();
        assert!(!config.pii_enabled);
        assert!((config.toxicity_threshold - 0.8).abs() < f32::EPSILON);

        // With PII disabled, email text should pass
        let engine = GuardrailsEngine::with_config(config);
        let result = engine.check("Email: test@example.com");
        assert!(result.pii_found.is_empty());
    }

    #[test]
    fn test_check_timing() {
        let engine = GuardrailsEngine::new();
        let result = engine.check("Some text to check for timing.");
        // check_time_us should be populated (non-negative, which it always is for u64)
        // Just verify it runs without panic and returns a value
        assert!(result.check_time_us < 10_000_000); // less than 10 seconds
    }
}
