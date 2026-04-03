//! GuardrailsEngine — orchestrates PII detection, toxicity checking, and custom rules.

use std::time::Instant;

use serde::{Deserialize, Serialize};
use vil_macros::VilAiEvent;

use crate::config::GuardrailsConfig;
use crate::pii::{PiiDetector, PiiMatch};
use crate::rules::{RuleAction, RuleEngine, RuleMatch};
use crate::toxicity::ToxicityChecker;

/// A violation detected by the guardrails engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    /// Category of violation.
    pub category: String,
    /// Human-readable description.
    pub description: String,
    /// Severity (0.0 = info, 1.0 = critical).
    pub severity: f32,
}

/// Result of a guardrails check.
#[derive(Debug, Clone, Serialize, Deserialize, VilAiEvent)]
pub struct GuardrailResult {
    /// Whether the content passed all checks.
    pub passed: bool,
    /// List of violations found.
    pub violations: Vec<Violation>,
    /// PII instances found.
    pub pii_found: Vec<PiiMatch>,
    /// Toxicity score (0.0 - 1.0).
    pub toxicity_score: f32,
    /// Rule matches.
    pub rule_matches: Vec<RuleMatch>,
    /// Check duration in microseconds.
    pub check_time_ns: u64,
}

/// The main guardrails engine combining PII, toxicity, and custom rules.
pub struct GuardrailsEngine {
    pub pii: PiiDetector,
    pub toxicity: ToxicityChecker,
    pub rules: RuleEngine,
    pub config: GuardrailsConfig,
}

impl GuardrailsEngine {
    /// Create a new engine with default configuration.
    pub fn new() -> Self {
        Self {
            pii: PiiDetector::new(),
            toxicity: ToxicityChecker::with_defaults(),
            rules: RuleEngine::new(),
            config: GuardrailsConfig::default(),
        }
    }

    /// Create with a specific configuration.
    pub fn with_config(config: GuardrailsConfig) -> Self {
        Self {
            pii: PiiDetector::new(),
            toxicity: ToxicityChecker::with_defaults(),
            rules: RuleEngine::new(),
            config,
        }
    }

    /// Run all enabled checks on the given text.
    pub fn check(&self, text: &str) -> GuardrailResult {
        let start = Instant::now();
        let mut violations = Vec::new();
        let mut passed = true;

        // PII detection
        let pii_found = if self.config.pii_enabled {
            let pii = self.pii.detect(text);
            if !pii.is_empty() {
                passed = false;
                violations.push(Violation {
                    category: "pii".to_string(),
                    description: format!("Found {} PII instance(s)", pii.len()),
                    severity: 0.8,
                });
            }
            pii
        } else {
            Vec::new()
        };

        // Toxicity scoring
        let toxicity_score = if self.config.toxicity_enabled {
            let score = self.toxicity.score(text);
            if score > self.config.toxicity_threshold {
                passed = false;
                violations.push(Violation {
                    category: "toxicity".to_string(),
                    description: format!(
                        "Toxicity score {:.2} exceeds threshold {:.2}",
                        score, self.config.toxicity_threshold
                    ),
                    severity: score,
                });
            }
            score
        } else {
            0.0
        };

        // Custom rules
        let rule_matches = if self.config.rules_enabled {
            let matches = self.rules.check(text);
            for m in &matches {
                match m.action {
                    RuleAction::Deny => {
                        passed = false;
                        violations.push(Violation {
                            category: "rule".to_string(),
                            description: format!(
                                "Rule '{}' denied: '{}'",
                                m.rule_name, m.matched_text
                            ),
                            severity: 0.9,
                        });
                    }
                    RuleAction::Allow => {
                        // Allow rules can override — handled at caller level
                    }
                    RuleAction::Redact => {
                        violations.push(Violation {
                            category: "rule".to_string(),
                            description: format!(
                                "Rule '{}' requires redaction: '{}'",
                                m.rule_name, m.matched_text
                            ),
                            severity: 0.5,
                        });
                    }
                }
            }
            matches
        } else {
            Vec::new()
        };

        let elapsed = start.elapsed();

        GuardrailResult {
            passed,
            violations,
            pii_found,
            toxicity_score,
            rule_matches,
            check_time_ns: elapsed.as_nanos() as u64,
        }
    }
}

impl Default for GuardrailsEngine {
    fn default() -> Self {
        Self::new()
    }
}
