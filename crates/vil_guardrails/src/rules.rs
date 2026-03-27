//! Custom rule engine for content guardrails.

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Action to take when a rule matches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuleAction {
    /// Allow the content (override other denials).
    Allow,
    /// Deny the content.
    Deny,
    /// Redact the matched portion.
    Redact,
}

/// A custom guardrail rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Human-readable name for this rule.
    pub name: String,
    /// Regex pattern to match.
    pub pattern: String,
    /// Action to take on match.
    pub action: RuleAction,
}

/// Compiled rule with pre-built regex.
struct CompiledRule {
    name: String,
    regex: Regex,
    action: RuleAction,
}

/// Result of a rule check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMatch {
    /// Name of the matching rule.
    pub rule_name: String,
    /// The action specified.
    pub action: RuleAction,
    /// Matched text.
    pub matched_text: String,
    /// Position in the original text.
    pub position: usize,
}

/// Custom rule engine with allow/deny/redact rules.
pub struct RuleEngine {
    rules: Vec<CompiledRule>,
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleEngine {
    /// Create an empty rule engine.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a rule. Returns Err if the pattern is invalid regex.
    pub fn add_rule(&mut self, rule: Rule) -> Result<(), regex::Error> {
        let regex = Regex::new(&rule.pattern)?;
        self.rules.push(CompiledRule {
            name: rule.name,
            regex,
            action: rule.action,
        });
        Ok(())
    }

    /// Check text against all rules. Returns all matches.
    pub fn check(&self, text: &str) -> Vec<RuleMatch> {
        let mut matches = Vec::new();
        for rule in &self.rules {
            for m in rule.regex.find_iter(text) {
                matches.push(RuleMatch {
                    rule_name: rule.name.clone(),
                    action: rule.action.clone(),
                    matched_text: m.as_str().to_string(),
                    position: m.start(),
                });
            }
        }
        matches.sort_by_key(|m| m.position);
        matches
    }

    /// Check if any deny rule matches.
    pub fn has_deny(&self, text: &str) -> bool {
        self.check(text)
            .iter()
            .any(|m| m.action == RuleAction::Deny)
    }

    /// Check if any allow rule matches (override).
    pub fn has_allow(&self, text: &str) -> bool {
        self.check(text)
            .iter()
            .any(|m| m.action == RuleAction::Allow)
    }

    /// Apply redaction rules and return modified text.
    pub fn redact(&self, text: &str) -> String {
        let mut result = text.to_string();
        for rule in &self.rules {
            if rule.action == RuleAction::Redact {
                result = rule.regex.replace_all(&result, "[REDACTED]").to_string();
            }
        }
        result
    }
}
